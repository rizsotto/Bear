// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::intercept;
use crate::intercept::supervise::SuperviseError;
use crate::intercept::tcp::ReceivingError;
use crate::output::FormatError;
use crossbeam_channel::{bounded, unbounded, Receiver};
use std::process::{ExitCode, ExitStatus};
use std::sync::Arc;
use thiserror::Error;

/// A trait for consuming events from a channel-based stream.
///
/// # Thread Safety
/// Implementers must be `Send` to allow usage across thread boundaries.
#[cfg_attr(test, mockall::automock)]
pub trait Consumer: Send {
    /// Consumes all items from the receiver until the channel is closed.
    ///
    /// This is a blocking operation that processes each received item.
    /// The method returns when the sender side of the channel is dropped
    /// or when an error occurs during processing.
    ///
    /// # Arguments
    /// * `receiver` - Channel receiver to consume items from
    ///
    /// # Returns
    /// * `Ok(())` - All items were successfully processed
    /// * `Err(FormatError)` - An error occurred during processing
    fn consume(self: Box<Self>, receiver: Receiver<intercept::Event>) -> Result<(), FormatError>;
}

/// A trait for producing events to a channel-based stream.
///
/// # Thread Safety
/// Implementers must be `Send + Sync` to allow usage across thread boundaries.
#[cfg_attr(test, mockall::automock)]
pub trait Producer: Send + Sync {
    /// Produces items by sending them through the provided sender.
    ///
    /// This is a blocking operation that continues until all items are produced
    /// or an error occurs. The producer should close the sender when finished
    /// to signal completion to consumers.
    ///
    /// # Arguments
    /// * `sender` - Channel sender to produce items to
    ///
    /// # Returns
    /// * `Ok(())` - All items were successfully produced
    /// * `Err(ReceivingError)` - An error occurred during production
    fn produce(
        &self,
        sender: crossbeam_channel::Sender<intercept::Event>,
    ) -> Result<(), ReceivingError>;
}

/// A trait for cancelling ongoing operations.
///
/// # Thread Safety
/// Implementers must be `Send + Sync` to allow usage across thread boundaries.
#[cfg_attr(test, mockall::automock)]
pub trait Cancellable: Send + Sync {
    /// Cancels the ongoing operation.
    ///
    /// # Returns
    /// * `Ok(())` - Cancellation was successful
    /// * `Err(ReceivingError)` - An error occurred during cancellation
    fn cancel(&self) -> Result<(), ReceivingError>;
}

/// A trait for producers that support cancellation during operation.
///
/// Combines `Producer` and `Cancellable` functionality for event production
/// that can be cancelled mid-operation.
pub trait CancellableProducer: Producer + Cancellable {}

/// A trait for executing build commands.
///
/// Executors are responsible for running the actual build process while
/// allowing command interception to occur. They manage the lifecycle of
/// the build command and report its exit status.
///
/// May encounter `SuperviseError` during execution.
#[cfg_attr(test, mockall::automock)]
pub trait Executor {
    /// Executes the given build command.
    ///
    /// This is a blocking operation that runs the build command to completion.
    /// During execution, the command and its subprocesses may be intercepted
    /// by Bear's interception mechanisms.
    ///
    /// # Arguments
    /// * `command` - The build command to execute
    ///
    /// # Returns
    /// * `Ok(ExitStatus)` - The build completed with the given exit status
    /// * `Err(SuperviseError)` - An error occurred during execution
    fn run(&self, command: BuildCommand) -> Result<ExitStatus, SuperviseError>;
}

/// Coordinates live command interception during build execution.
///
/// `Interceptor` manages the simultaneous execution of:
/// - Build command execution (via `Executor`)
/// - Command interception (via `CancellableProducer`)
/// - Event processing (via `Consumer`)
///
/// The interceptor ensures proper coordination between these components,
/// handling thread synchronization and error propagation.
pub struct Interceptor {
    producer: Arc<dyn CancellableProducer>,
    consumer: Box<dyn Consumer>,
    build: Box<dyn Executor>,
}

impl Interceptor {
    pub fn new(
        producer: Arc<dyn CancellableProducer>,
        consumer: Box<dyn Consumer>,
        build: Box<dyn Executor>,
    ) -> Self {
        Self {
            producer,
            consumer,
            build,
        }
    }

    /// Runs live command interception for the given build command.
    ///
    /// # Arguments
    /// * `command` - The build command to execute with interception
    ///
    /// # Returns
    /// * `Ok(ExitCode::SUCCESS)` - All operations completed successfully
    /// * `Err(RuntimeError)` - An error occurred in any component
    pub fn run(self, command: BuildCommand) -> Result<ExitCode, RuntimeError> {
        let (sender, receiver) = unbounded::<intercept::Event>();

        let producer_thread = {
            let producer = Arc::clone(&self.producer);
            std::thread::spawn(move || producer.produce(sender))
        };

        let consumer_thread = {
            let consumer = self.consumer;
            std::thread::spawn(move || consumer.consume(receiver))
        };

        let exit_status = self.build.run(command)?;

        self.producer.cancel()?;

        // Handle the producer thread result
        producer_thread
            .join()
            .map_err(|_| RuntimeError::Thread("Source thread panicked"))?
            .map_err(RuntimeError::Producer)?;

        // Handle the consumer thread result
        consumer_thread
            .join()
            .map_err(|_| RuntimeError::Thread("Consumer thread panicked"))?
            .map_err(RuntimeError::Consumer)?;

        // The exit code is not always available. When the process is killed by a signal,
        // the exit code is not available. In this case, we return the `FAILURE` exit code.
        let exit_code = exit_status
            .code()
            .map(|code| ExitCode::from(code as u8))
            .unwrap_or(ExitCode::FAILURE);

        Ok(exit_code)
    }
}

/// Replays previously captured intercept events.
///
/// `Replayer` processes stored intercept events without executing a build command.
/// This is useful for:
/// - Re-analyzing previous builds with different configurations
/// - Testing semantic analysis changes
/// - Generating compilation databases from archived event data
pub struct Replayer {
    source: Box<dyn Producer>,
    consumer: Box<dyn Consumer>,
}

impl Replayer {
    pub fn new(source: Box<dyn Producer>, consumer: Box<dyn Consumer>) -> Self {
        Self { source, consumer }
    }

    /// Replays stored intercept events through the processing pipeline.
    ///
    /// # Returns
    /// * `Ok(ExitCode::SUCCESS)` - All events were successfully replayed
    /// * `Err(RuntimeError)` - An error occurred during replay (most likely IO error)
    pub fn run(self) -> Result<ExitCode, RuntimeError> {
        // Using bounded channel to reduce memory usage and implement backpressure.
        // This is possible with replay mode, because the source is a file. While this
        // is not possible with intercept mode, because that would slow the build process.
        let (sender, receiver) = bounded::<intercept::Event>(10);

        let source_thread = {
            let source = self.source;
            std::thread::spawn(move || source.produce(sender))
        };

        let consumer_thread = {
            let consumer = self.consumer;
            std::thread::spawn(move || consumer.consume(receiver))
        };

        // Handle the source thread result
        source_thread
            .join()
            .map_err(|_| RuntimeError::Thread("Source thread panicked"))?
            .map_err(RuntimeError::Producer)?;

        // Handle the consumer thread result
        consumer_thread
            .join()
            .map_err(|_| RuntimeError::Thread("Consumer thread panicked"))?
            .map_err(RuntimeError::Consumer)?;

        Ok(ExitCode::SUCCESS)
    }
}

/// Errors that can occur during event processing or running the build.
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Producer error: {0}")]
    Producer(#[from] ReceivingError),
    #[error("Consumer error: {0}")]
    Consumer(#[from] FormatError),
    #[error("Executor error: {0}")]
    Executor(#[from] SuperviseError),
    #[error("Thread error: {0}")]
    Thread(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intercept::Event;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // Helper functions for comparing ExitCode values that works with older Rust versions
    fn assert_is_success(code: ExitCode) {
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    fn assert_is_failure(code: ExitCode) {
        // Use inequality with SUCCESS rather than equality with FAILURE
        // to avoid platform-specific differences in failure representation
        assert_ne!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    // Simple mock struct that implements both Producer and Cancellable
    struct MockCancellableProducer {
        events: Vec<Event>,
        should_fail_produce: bool,
        should_fail_cancel: bool,
        cancel_count: Arc<Mutex<usize>>,
    }

    impl MockCancellableProducer {
        fn new(events: Vec<Event>) -> Self {
            Self {
                events,
                should_fail_produce: false,
                should_fail_cancel: false,
                cancel_count: Arc::new(Mutex::new(0)),
            }
        }

        fn with_produce_failure(events: Vec<Event>) -> Self {
            Self {
                events,
                should_fail_produce: true,
                should_fail_cancel: false,
                cancel_count: Arc::new(Mutex::new(0)),
            }
        }

        fn with_cancel_failure(events: Vec<Event>) -> Self {
            Self {
                events,
                should_fail_produce: false,
                should_fail_cancel: true,
                cancel_count: Arc::new(Mutex::new(0)),
            }
        }

        fn cancel_call_count(&self) -> usize {
            *self
                .cancel_count
                .lock()
                .expect("Failed to lock cancel_count mutex")
        }
    }

    impl Producer for MockCancellableProducer {
        fn produce(
            &self,
            sender: crossbeam_channel::Sender<intercept::Event>,
        ) -> Result<(), ReceivingError> {
            if self.should_fail_produce {
                return Err(ReceivingError::Network(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "Test failure",
                )));
            }

            for event in &self.events {
                sender.send(event.clone()).map_err(|_| {
                    ReceivingError::Network(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "Channel disconnected",
                    ))
                })?;
            }
            Ok(())
        }
    }

    impl Cancellable for MockCancellableProducer {
        fn cancel(&self) -> Result<(), ReceivingError> {
            *self
                .cancel_count
                .lock()
                .expect("Failed to lock cancel_count mutex") += 1;
            if self.should_fail_cancel {
                return Err(ReceivingError::Network(std::io::Error::other(
                    "Cancel failure",
                )));
            }
            Ok(())
        }
    }

    impl CancellableProducer for MockCancellableProducer {}

    fn create_test_event(pid: u32, executable: &str) -> Event {
        Event::from_strings(
            pid,
            executable,
            vec!["arg1", "arg2"],
            "/tmp",
            HashMap::new(),
        )
    }

    fn create_test_command() -> BuildCommand {
        BuildCommand {
            arguments: vec!["make".to_string(), "all".to_string()],
        }
    }

    fn create_success_exit_status() -> ExitStatus {
        std::process::Command::new("true")
            .status()
            .expect("Failed to get success exit status")
    }

    fn create_failure_exit_status() -> ExitStatus {
        std::process::Command::new("false")
            .status()
            .expect("Failed to get failure exit status")
    }

    #[test]
    fn test_replayer_happy_path() {
        let events = vec![
            create_test_event(1001, "/usr/bin/gcc"),
            create_test_event(1002, "/usr/bin/clang"),
            create_test_event(1003, "/usr/bin/g++"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let mut producer_mock = MockProducer::new();
        producer_mock
            .expect_produce()
            .times(1)
            .returning(move |sender| {
                let test_events = vec![
                    create_test_event(1001, "/usr/bin/gcc"),
                    create_test_event(1002, "/usr/bin/clang"),
                    create_test_event(1003, "/usr/bin/g++"),
                ];
                for event in test_events {
                    sender.send(event).expect("Failed to send test event");
                }
                Ok(())
            });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(move |receiver| {
                for event in receiver {
                    captured_events_clone
                        .lock()
                        .expect("Failed to lock captured_events mutex")
                        .push(event);
                }
                Ok(())
            });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in happy path test"));

        let consumed_events = captured_events
            .lock()
            .expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 3);
        assert_eq!(*consumed_events, events);
    }

    #[test]
    fn test_replayer_producer_failure() {
        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|_| {
            Err(ReceivingError::Network(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "Test failure",
            )))
        });

        let consumer_mock = MockConsumer::new();
        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            RuntimeError::Producer(ReceivingError::Network(_))
        );
    }

    #[test]
    fn test_replayer_consumer_failure() {
        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|sender| {
            sender
                .send(create_test_event(1001, "/usr/bin/gcc"))
                .expect("Failed to send test event");
            Ok(())
        });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(|_| Err(FormatError::Io(std::io::Error::other("Test failure"))));

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_err());
        matches!(result.unwrap_err(), RuntimeError::Consumer(_));
    }

    #[test]
    fn test_replayer_empty_events() {
        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let mut producer_mock = MockProducer::new();
        producer_mock
            .expect_produce()
            .times(1)
            .returning(|_| Ok(()));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(move |receiver| {
                for event in receiver {
                    captured_events_clone
                        .lock()
                        .expect("Failed to lock captured_events mutex")
                        .push(event);
                }
                Ok(())
            });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in empty events test"));
        assert_eq!(
            captured_events
                .lock()
                .expect("Failed to lock captured_events mutex")
                .len(),
            0
        );
    }

    #[test]
    fn test_interceptor_happy_path() {
        let events = vec![
            create_test_event(2001, "/usr/bin/gcc"),
            create_test_event(2002, "/usr/bin/clang"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let producer_mock = Arc::new(MockCancellableProducer::new(events.clone()));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(move |receiver| {
                for event in receiver {
                    captured_events_clone
                        .lock()
                        .expect("Failed to lock captured_events mutex")
                        .push(event);
                }
                Ok(())
            });

        let mut executor_mock = MockExecutor::new();
        executor_mock
            .expect_run()
            .times(1)
            .returning(|_| Ok(create_success_exit_status()));

        let interceptor = Interceptor::new(
            producer_mock.clone(),
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in interceptor happy path test"));

        let consumed_events = captured_events
            .lock()
            .expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 2);
        assert_eq!(*consumed_events, events);
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_executor_failure() {
        let producer_mock = Arc::new(MockCancellableProducer::new(vec![]));
        let consumer_mock = MockConsumer::new();

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| {
            Err(SuperviseError::ProcessSpawn(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Test executor failure",
            )))
        });

        let interceptor = Interceptor::new(
            producer_mock,
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        matches!(result.unwrap_err(), RuntimeError::Executor(_));
    }

    #[test]
    fn test_interceptor_producer_failure() {
        let events = vec![create_test_event(2001, "/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::with_produce_failure(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(|_| Ok(()));

        let mut executor_mock = MockExecutor::new();
        executor_mock
            .expect_run()
            .times(1)
            .returning(|_| Ok(create_success_exit_status()));

        let interceptor = Interceptor::new(
            producer_mock.clone(),
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        matches!(result.unwrap_err(), RuntimeError::Producer(_));
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_consumer_failure() {
        let events = vec![create_test_event(2001, "/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::new(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(|_| Err(FormatError::Io(std::io::Error::other("Test failure"))));

        let mut executor_mock = MockExecutor::new();
        executor_mock
            .expect_run()
            .times(1)
            .returning(|_| Ok(create_success_exit_status()));

        let interceptor = Interceptor::new(
            producer_mock.clone(),
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        matches!(result.unwrap_err(), RuntimeError::Consumer(_));
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_cancel_failure() {
        let events = vec![create_test_event(2001, "/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::with_cancel_failure(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(|_| Ok(()));

        let mut executor_mock = MockExecutor::new();
        executor_mock
            .expect_run()
            .times(1)
            .returning(|_| Ok(create_success_exit_status()));

        let interceptor = Interceptor::new(
            producer_mock.clone(),
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        matches!(result.unwrap_err(), RuntimeError::Producer(_));
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_non_zero_exit_code() {
        let events = vec![create_test_event(2001, "/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::new(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(|_| Ok(()));

        let mut executor_mock = MockExecutor::new();
        executor_mock
            .expect_run()
            .times(1)
            .returning(|_| Ok(create_failure_exit_status()));

        let interceptor = Interceptor::new(
            producer_mock,
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_ok());
        assert_is_failure(result.expect("Failed to get result in non-zero exit code test"));
    }

    #[test]
    fn test_interceptor_coordination_timing() {
        let events = vec![
            create_test_event(3001, "/usr/bin/gcc"),
            create_test_event(3002, "/usr/bin/clang"),
            create_test_event(3003, "/usr/bin/g++"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let producer_mock = Arc::new(MockCancellableProducer::new(events.clone()));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(move |receiver| {
                for event in receiver {
                    std::thread::sleep(Duration::from_millis(5));
                    captured_events_clone
                        .lock()
                        .expect("Failed to lock captured_events mutex")
                        .push(event);
                }
                Ok(())
            });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| {
            std::thread::sleep(Duration::from_millis(50));
            Ok(create_success_exit_status())
        });

        let interceptor = Interceptor::new(
            producer_mock.clone(),
            Box::new(consumer_mock),
            Box::new(executor_mock),
        );
        let result = interceptor.run(create_test_command());

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in coordination timing test"));

        let consumed_events = captured_events
            .lock()
            .expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 3);
        assert_eq!(*consumed_events, events);
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_replayer_coordination_timing() {
        let events = vec![
            create_test_event(4001, "/usr/bin/gcc"),
            create_test_event(4002, "/usr/bin/clang"),
            create_test_event(4003, "/usr/bin/g++"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let mut producer_mock = MockProducer::new();
        producer_mock
            .expect_produce()
            .times(1)
            .returning(move |sender| {
                let test_events = vec![
                    create_test_event(4001, "/usr/bin/gcc"),
                    create_test_event(4002, "/usr/bin/clang"),
                    create_test_event(4003, "/usr/bin/g++"),
                ];
                for event in test_events {
                    std::thread::sleep(Duration::from_millis(5));
                    sender.send(event).expect("Failed to send test event");
                }
                Ok(())
            });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock
            .expect_consume()
            .times(1)
            .returning(move |receiver| {
                for event in receiver {
                    std::thread::sleep(Duration::from_millis(10));
                    captured_events_clone
                        .lock()
                        .expect("Failed to lock captured_events mutex")
                        .push(event);
                }
                Ok(())
            });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));

        let result = replayer.run();

        assert!(result.is_ok());
        assert_is_success(
            result.expect("Failed to get result in replayer coordination timing test"),
        );

        let consumed_events = captured_events
            .lock()
            .expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 3);
        assert_eq!(*consumed_events, events);
    }
}

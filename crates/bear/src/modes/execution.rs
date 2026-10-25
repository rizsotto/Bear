// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::environment::execution_from_build_command;
use crate::output::AtomicFile;
use intercept_supervisor::SuperviseError;
use std::process::{ExitCode, ExitStatus};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use thiserror::Error;

/// A type-erased error carried across the pipeline seam.
///
/// Each `Producer`/`Consumer`/`Cancellable` implementation keeps its own
/// precise error type internally and boxes it here at the trait boundary, so
/// the traits do not force every implementation onto one shared error type.
pub type DynError = Box<dyn std::error::Error + Send + Sync>;

/// Backpressure here would slow the build: a full channel stops the
/// collector from accepting reports. The room is large enough that only a
/// consumer far behind the build ever blocks it.
const INTERCEPT_CHANNEL_CAPACITY: usize = 1024;

/// Replay reads from a file or a pipe, so backpressure costs nothing and keeps memory
/// use low.
const REPLAY_CHANNEL_CAPACITY: usize = 10;

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
    /// The consumer writes its output but does not publish it: only the
    /// caller knows whether the whole run succeeded, so it commits the
    /// returned file or drops it to roll it back.
    ///
    /// # Arguments
    /// * `receiver` - Channel receiver to consume items from
    ///
    /// # Returns
    /// * `Ok(AtomicFile)` - All items were written, the output waits for the commit
    /// * `Err(DynError)` - An error occurred during processing
    fn consume(self: Box<Self>, receiver: Receiver<intercept::Execution>) -> Result<AtomicFile, DynError>;
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
    /// * `Err(DynError)` - An error occurred during production
    fn produce(&self, sender: SyncSender<intercept::Execution>) -> Result<(), DynError>;
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
    /// * `Err(DynError)` - An error occurred during cancellation
    fn cancel(&self) -> Result<(), DynError>;
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
        Self { producer, consumer, build }
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
        let (sender, receiver) = sync_channel::<intercept::Execution>(INTERCEPT_CHANNEL_CAPACITY);

        // Inject the initial command as the first execution
        let _ = sender.send(execution_from_build_command(&command));

        let producer_thread = {
            let producer = Arc::clone(&self.producer);
            std::thread::spawn(move || producer.produce(sender))
        };

        let consumer_thread = {
            let consumer = self.consumer;
            std::thread::spawn(move || consumer.consume(receiver))
        };

        let build_result = self.build.run(command);

        // Stop the interception whatever the build did, so both threads
        // finish and the output can be committed or rolled back. A failed
        // cancel may leave the producer running forever, so the threads are
        // not joined then, and the output is left as it is.
        if let Err(error) = self.producer.cancel() {
            // Report the build error first: it is the likelier cause.
            build_result?;
            return Err(RuntimeError::Producer(error));
        }

        let producer_result =
            producer_thread.join().map_err(|_| RuntimeError::Thread("Producer thread panicked"));
        let consumer_result =
            consumer_thread.join().map_err(|_| RuntimeError::Thread("Consumer thread panicked"));

        // Publish only when nothing failed on Bear's side; every early return
        // drops the output, which rolls it back. A build that ran and failed
        // is not such a failure: its exit status is returned below.
        let exit_status = build_result?;
        producer_result?.map_err(RuntimeError::Producer)?;
        commit(consumer_result?.map_err(RuntimeError::Consumer)?)?;

        // The exit code is not always available. When the process is killed by a signal,
        // the exit code is not available. In this case, we return the `FAILURE` exit code.
        let exit_code =
            exit_status.code().map(|code| ExitCode::from(code as u8)).unwrap_or(ExitCode::FAILURE);

        Ok(exit_code)
    }
}

/// Drives a producer/consumer pair over existing input, without running a build.
///
/// `Replayer` powers the modes that process input that already exists:
/// - Re-analyzing previously captured intercept events (with different configurations)
/// - Testing semantic analysis changes against archived event data
/// - Interpreting shell command text into execution events (parse-sh)
pub struct Replayer {
    producer: Box<dyn Producer>,
    consumer: Box<dyn Consumer>,
}

impl Replayer {
    pub fn new(producer: Box<dyn Producer>, consumer: Box<dyn Consumer>) -> Self {
        Self { producer, consumer }
    }

    /// Replays stored intercept events through the processing pipeline.
    ///
    /// # Returns
    /// * `Ok(ExitCode::SUCCESS)` - All events were successfully replayed
    /// * `Err(RuntimeError)` - An error occurred during replay (most likely IO error)
    pub fn run(self) -> Result<ExitCode, RuntimeError> {
        let (sender, receiver) = sync_channel::<intercept::Execution>(REPLAY_CHANNEL_CAPACITY);

        let producer_thread = {
            let producer = self.producer;
            std::thread::spawn(move || producer.produce(sender))
        };

        let consumer_thread = {
            let consumer = self.consumer;
            std::thread::spawn(move || consumer.consume(receiver))
        };

        // Join both threads before propagating either error, then publish
        // only when both succeeded; every early return drops the output,
        // which rolls it back. The producer's error wins when both fail,
        // since the consumer only saw a truncated stream of it.
        let producer_result =
            producer_thread.join().map_err(|_| RuntimeError::Thread("Producer thread panicked"));
        let consumer_result =
            consumer_thread.join().map_err(|_| RuntimeError::Thread("Consumer thread panicked"));

        producer_result?.map_err(RuntimeError::Producer)?;
        commit(consumer_result?.map_err(RuntimeError::Consumer)?)?;

        Ok(ExitCode::SUCCESS)
    }
}

fn commit(output: AtomicFile) -> Result<(), RuntimeError> {
    output.commit().map_err(|err| RuntimeError::Consumer(Box::new(err)))
}

/// Errors that can occur during event processing or running the build.
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("{0}")]
    Producer(DynError),
    #[error("{0}")]
    Consumer(DynError),
    #[error(transparent)]
    Executor(#[from] SuperviseError),
    #[error("Internal error: {0}")]
    Thread(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{SerializationError, WriterError};
    use intercept::Execution;
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
        executions: Vec<Execution>,
        should_fail_produce: bool,
        should_fail_cancel: bool,
        cancel_count: Arc<Mutex<usize>>,
    }

    impl MockCancellableProducer {
        fn new(executions: Vec<Execution>) -> Self {
            Self {
                executions,
                should_fail_produce: false,
                should_fail_cancel: false,
                cancel_count: Arc::new(Mutex::new(0)),
            }
        }

        fn with_produce_failure(executions: Vec<Execution>) -> Self {
            Self {
                executions,
                should_fail_produce: true,
                should_fail_cancel: false,
                cancel_count: Arc::new(Mutex::new(0)),
            }
        }

        fn with_cancel_failure(executions: Vec<Execution>) -> Self {
            Self {
                executions,
                should_fail_produce: false,
                should_fail_cancel: true,
                cancel_count: Arc::new(Mutex::new(0)),
            }
        }

        fn cancel_call_count(&self) -> usize {
            *self.cancel_count.lock().expect("Failed to lock cancel_count mutex")
        }
    }

    impl Producer for MockCancellableProducer {
        fn produce(&self, sender: SyncSender<intercept::Execution>) -> Result<(), DynError> {
            if self.should_fail_produce {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "Test failure",
                )));
            }

            for execution in &self.executions {
                // Mirror the real producers: a disconnected channel means the
                // consumer finished or failed, so stop quietly instead of
                // erroring (which would race the consumer's own error).
                if sender.send(execution.clone()).is_err() {
                    break;
                }
            }
            Ok(())
        }
    }

    impl Cancellable for MockCancellableProducer {
        fn cancel(&self) -> Result<(), DynError> {
            *self.cancel_count.lock().expect("Failed to lock cancel_count mutex") += 1;
            if self.should_fail_cancel {
                return Err(Box::new(std::io::Error::other("Cancel failure")));
            }
            Ok(())
        }
    }

    impl CancellableProducer for MockCancellableProducer {}

    fn create_test_execution(executable: &str) -> Execution {
        Execution::from_strings(executable, vec!["arg1", "arg2"], "/tmp", HashMap::new())
    }

    fn create_test_command() -> BuildCommand {
        BuildCommand { arguments: vec!["make".to_string(), "all".to_string()] }
    }

    fn create_success_exit_status() -> ExitStatus {
        // According to the Rust documentation, ExitStatus::default() is equivalent to success.
        ExitStatus::default()
    }

    #[cfg(unix)]
    fn create_failure_exit_status() -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatusExt::from_raw(256) // On Unix, 256 represents exit code 1 (256 >> 8 = 1)
    }

    #[cfg(not(unix))]
    fn create_failure_exit_status() -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatusExt::from_raw(1) // On Windows, use raw value 1 to get exit code 1
    }

    #[test]
    fn test_replayer_happy_path() {
        let events = vec![
            create_test_execution("/usr/bin/gcc"),
            create_test_execution("/usr/bin/clang"),
            create_test_execution("/usr/bin/g++"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(move |sender| {
            let test_events = vec![
                create_test_execution("/usr/bin/gcc"),
                create_test_execution("/usr/bin/clang"),
                create_test_execution("/usr/bin/g++"),
            ];
            for event in test_events {
                sender.send(event).expect("Failed to send test event");
            }
            Ok(())
        });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(move |receiver| {
            for event in receiver {
                captured_events_clone.lock().expect("Failed to lock captured_events mutex").push(event);
            }
            Ok(AtomicFile::none())
        });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in happy path test"));

        let consumed_events = captured_events.lock().expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 3);
        assert_eq!(*consumed_events, events);
    }

    #[test]
    fn test_replayer_producer_failure() {
        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|_| {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Test failure")))
        });

        let consumer_mock = MockConsumer::new();
        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Producer(_)));
    }

    #[test]
    fn test_replayer_consumer_failure() {
        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|sender| {
            // The consumer fails without receiving, so the send may observe
            // a dropped receiver; the producer must not turn that into its
            // own failure, or the asserted error variant becomes racy.
            let _ = sender.send(create_test_execution("/usr/bin/gcc"));
            Ok(())
        });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|_| {
            Err(Box::new(WriterError::Io(
                std::path::PathBuf::new(),
                SerializationError::Io(std::io::Error::other("Test failure")),
            )))
        });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Consumer(_)));
    }

    #[test]
    fn test_replayer_producer_stops_when_consumer_drops_receiver_mid_stream() {
        // More events than the channel holds, so the producer blocks in send
        // until the consumer drops its receiver.
        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|sender| {
            for _ in 0..(REPLAY_CHANNEL_CAPACITY * 5) {
                if sender.send(create_test_execution("/usr/bin/gcc")).is_err() {
                    return Ok(());
                }
            }
            panic!("the consumer dropped its receiver, so a send must fail");
        });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|receiver| {
            receiver.recv().expect("first event");
            Err(Box::new(WriterError::Io(
                std::path::PathBuf::new(),
                SerializationError::Io(std::io::Error::other("Test failure")),
            )))
        });

        let sut = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock)).run();

        assert!(matches!(sut, Err(RuntimeError::Consumer(_))));
    }

    #[test]
    fn test_replayer_empty_events() {
        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|_| Ok(()));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(move |receiver| {
            for event in receiver {
                captured_events_clone.lock().expect("Failed to lock captured_events mutex").push(event);
            }
            Ok(AtomicFile::none())
        });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));
        let result = replayer.run();

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in empty events test"));
        assert_eq!(captured_events.lock().expect("Failed to lock captured_events mutex").len(), 0);
    }

    #[test]
    fn test_interceptor_happy_path() {
        let events = vec![create_test_execution("/usr/bin/gcc"), create_test_execution("/usr/bin/clang")];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let producer_mock = Arc::new(MockCancellableProducer::new(events.clone()));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(move |receiver| {
            for event in receiver {
                captured_events_clone.lock().expect("Failed to lock captured_events mutex").push(event);
            }
            Ok(AtomicFile::none())
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| Ok(create_success_exit_status()));

        let interceptor =
            Interceptor::new(producer_mock.clone(), Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in interceptor happy path test"));

        let consumed_events = captured_events.lock().expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 3);

        // Verify the first item is the initial command execution
        let initial = &consumed_events[0];
        assert_eq!(initial.executable.to_str().unwrap(), "make");
        assert_eq!(initial.arguments, vec!["make", "all"]);

        // Verify the remaining items match the mock producer executions
        assert_eq!(consumed_events[1..], events);
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_executor_failure() {
        let producer_mock = Arc::new(MockCancellableProducer::new(vec![]));
        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|receiver| {
            for _ in receiver {}
            Ok(AtomicFile::none())
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| {
            Err(SuperviseError::ProcessSpawn {
                executable: "test_executable".into(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Test executor failure"),
            })
        });

        let interceptor = Interceptor::new(producer_mock, Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Executor(_)));
    }

    #[test]
    fn test_interceptor_producer_failure() {
        let events = vec![create_test_execution("/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::with_produce_failure(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|receiver| {
            // Actually consume all events from the channel to prevent race condition
            // where the channel is closed before the producer finishes sending
            while receiver.recv().is_ok() {
                // Consume events until channel is closed
            }
            Ok(AtomicFile::none())
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| Ok(create_success_exit_status()));

        let interceptor =
            Interceptor::new(producer_mock.clone(), Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Producer(_)));
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_consumer_failure() {
        let events = vec![create_test_execution("/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::new(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|_| {
            Err(Box::new(WriterError::Io(
                std::path::PathBuf::new(),
                SerializationError::Io(std::io::Error::other("Test failure")),
            )))
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| Ok(create_success_exit_status()));

        let interceptor =
            Interceptor::new(producer_mock.clone(), Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Consumer(_)));
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_cancel_failure() {
        let events = vec![create_test_execution("/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::with_cancel_failure(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|receiver| {
            // Actually consume all events from the channel to prevent race condition
            // where the channel is closed before the producer finishes sending
            while receiver.recv().is_ok() {
                // Consume events until channel is closed
            }
            Ok(AtomicFile::none())
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| Ok(create_success_exit_status()));

        let interceptor =
            Interceptor::new(producer_mock.clone(), Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Producer(_)));
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_interceptor_non_zero_exit_code() {
        let events = vec![create_test_execution("/usr/bin/gcc")];
        let producer_mock = Arc::new(MockCancellableProducer::new(events));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(|receiver| {
            // Actually consume all events from the channel to prevent race condition
            // where the channel is closed before the producer finishes sending
            while receiver.recv().is_ok() {
                // Consume events until channel is closed
            }
            Ok(AtomicFile::none())
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| Ok(create_failure_exit_status()));

        let interceptor = Interceptor::new(producer_mock, Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_ok());
        assert_is_failure(result.expect("Failed to get result in non-zero exit code test"));
    }

    #[test]
    fn test_interceptor_coordination_timing() {
        let events = vec![
            create_test_execution("/usr/bin/gcc"),
            create_test_execution("/usr/bin/clang"),
            create_test_execution("/usr/bin/g++"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let producer_mock = Arc::new(MockCancellableProducer::new(events.clone()));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(move |receiver| {
            for event in receiver {
                std::thread::sleep(Duration::from_millis(5));
                captured_events_clone.lock().expect("Failed to lock captured_events mutex").push(event);
            }
            Ok(AtomicFile::none())
        });

        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| {
            std::thread::sleep(Duration::from_millis(50));
            Ok(create_success_exit_status())
        });

        let interceptor =
            Interceptor::new(producer_mock.clone(), Box::new(consumer_mock), Box::new(executor_mock));
        let result = interceptor.run(create_test_command());

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in coordination timing test"));

        let consumed_events = captured_events.lock().expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 4);

        // Verify the first item is the initial command execution
        let initial = &consumed_events[0];
        assert_eq!(initial.executable.to_str().unwrap(), "make");
        assert_eq!(initial.arguments, vec!["make", "all"]);

        // Verify the remaining items match the mock producer executions
        assert_eq!(consumed_events[1..], events);
        assert_eq!(producer_mock.cancel_call_count(), 1);
    }

    #[test]
    fn test_replayer_coordination_timing() {
        let events = vec![
            create_test_execution("/usr/bin/gcc"),
            create_test_execution("/usr/bin/clang"),
            create_test_execution("/usr/bin/g++"),
        ];

        let captured_events = Arc::new(Mutex::new(Vec::new()));
        let captured_events_clone = Arc::clone(&captured_events);

        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(move |sender| {
            let test_events = vec![
                create_test_execution("/usr/bin/gcc"),
                create_test_execution("/usr/bin/clang"),
                create_test_execution("/usr/bin/g++"),
            ];
            for event in test_events {
                std::thread::sleep(Duration::from_millis(5));
                sender.send(event).expect("Failed to send test event");
            }
            Ok(())
        });

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(move |receiver| {
            for event in receiver {
                std::thread::sleep(Duration::from_millis(10));
                captured_events_clone.lock().expect("Failed to lock captured_events mutex").push(event);
            }
            Ok(AtomicFile::none())
        });

        let replayer = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock));

        let result = replayer.run();

        assert!(result.is_ok());
        assert_is_success(result.expect("Failed to get result in replayer coordination timing test"));

        let consumed_events = captured_events.lock().expect("Failed to lock captured_events mutex");
        assert_eq!(consumed_events.len(), 3);
        assert_eq!(*consumed_events, events);
    }

    /// A consumer that drains the channel and hands back a pending file
    /// for `final_path`, as the real file-backed consumers do.
    fn consumer_with_pending_output(final_path: &std::path::Path) -> MockConsumer {
        let (output, file) = AtomicFile::create(final_path).expect("Failed to create the atomic file");
        drop(file);
        let output = Mutex::new(Some(output));

        let mut consumer_mock = MockConsumer::new();
        consumer_mock.expect_consume().times(1).returning(move |receiver| {
            for _ in receiver {}
            Ok(output.lock().expect("Failed to lock output mutex").take().expect("consume runs once"))
        });
        consumer_mock
    }

    // Requirements: output-atomic-write
    #[test]
    fn test_interceptor_rolls_back_when_build_cannot_start() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        let producer_mock = Arc::new(MockCancellableProducer::new(vec![]));
        let consumer_mock = consumer_with_pending_output(&final_path);
        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| {
            Err(SuperviseError::ProcessSpawn {
                executable: "test_executable".into(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Test executor failure"),
            })
        });

        let sut = Interceptor::new(producer_mock.clone(), Box::new(consumer_mock), Box::new(executor_mock))
            .run(create_test_command());

        assert!(matches!(sut, Err(RuntimeError::Executor(_))));
        assert_eq!(producer_mock.cancel_call_count(), 1);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "nothing may be left behind");
    }

    // Requirements: output-atomic-write
    #[test]
    fn test_interceptor_commits_when_build_fails() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        let producer_mock = Arc::new(MockCancellableProducer::new(vec![]));
        let consumer_mock = consumer_with_pending_output(&final_path);
        let mut executor_mock = MockExecutor::new();
        executor_mock.expect_run().times(1).returning(|_| Ok(create_failure_exit_status()));

        let sut = Interceptor::new(producer_mock, Box::new(consumer_mock), Box::new(executor_mock))
            .run(create_test_command());

        assert_is_failure(sut.expect("a failing build is not an error of Bear"));
        assert!(final_path.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "only the output may remain");
    }

    // Requirements: output-atomic-write
    #[test]
    fn test_replayer_rolls_back_when_producer_fails() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("output.json");
        let mut producer_mock = MockProducer::new();
        producer_mock.expect_produce().times(1).returning(|_| {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Test failure")))
        });
        let consumer_mock = consumer_with_pending_output(&final_path);

        let sut = Replayer::new(Box::new(producer_mock), Box::new(consumer_mock)).run();

        assert!(matches!(sut, Err(RuntimeError::Producer(_))));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "nothing may be left behind");
    }
}

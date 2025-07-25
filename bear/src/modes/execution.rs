// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::intercept;
use crate::intercept::executor;
use crate::intercept::supervise::SuperviseError;
use crate::intercept::{CancellableProducer, Producer, ProducerError};
use crate::output::FormatError;
use crossbeam_channel::{bounded, unbounded, Receiver};
use std::process::ExitCode;
use std::sync::Arc;
use thiserror::Error;

/// A trait for consuming events from a channel-based stream.
///
/// # Type Parameters
/// - `T`: The type of items being consumed (typically `intercept::Event`)
/// - `E`: The error type that can occur during consumption
///
/// # Thread Safety
/// Implementors must be `Send + Sync` to allow usage across thread boundaries.
pub trait Consumer<T, E>: Send {
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
    /// * `Err(E)` - An error occurred during processing
    fn consume(self: Box<Self>, _: Receiver<T>) -> Result<(), E>;
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
    producer: Arc<dyn CancellableProducer<intercept::Event, ProducerError>>,
    consumer: Box<dyn Consumer<intercept::Event, FormatError>>,
    build: Box<dyn executor::Executor<SuperviseError>>,
}

impl Interceptor {
    pub fn new(
        producer: Arc<dyn CancellableProducer<intercept::Event, ProducerError>>,
        consumer: Box<dyn Consumer<intercept::Event, FormatError>>,
        build: Box<dyn executor::Executor<SuperviseError>>,
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
    source: Box<dyn Producer<intercept::Event, ProducerError>>,
    consumer: Box<dyn Consumer<intercept::Event, FormatError>>,
}

impl Replayer {
    pub fn new(
        source: Box<dyn Producer<intercept::Event, ProducerError>>,
        consumer: Box<dyn Consumer<intercept::Event, FormatError>>,
    ) -> Self {
        Self { source, consumer }
    }

    /// Replays stored intercept events through the processing pipeline.
    ///
    /// # Returns
    /// * `Ok(ExitCode::SUCCESS)` - All events were successfully replayed
    /// * `Err(RuntimeError)` - An error occurred during replay (most likely IO error)
    pub fn run(self) -> Result<ExitCode, RuntimeError> {
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
    Producer(#[from] ProducerError),

    #[error("Consumer error: {0}")]
    Consumer(#[from] FormatError),

    #[error("Executor error: {0}")]
    Executor(#[from] SuperviseError),

    #[error("Thread error: {0}")]
    Thread(&'static str),
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! # Execution Modes
//!
//! This module provides the core execution patterns for Bear's operation modes.
//! It defines traits and implementations for the producer-consumer pattern
//! used throughout the application.

mod execution;

use crate::intercept::environment;
use crate::intercept::tcp::CollectorOnTcp;
use crate::{args, config, context, output};
use std::process::ExitCode;
use std::sync::Arc;

/// Represents the application execution modes.
///
/// Bear supports three user-facing modes:
/// - **Intercept only**: Capture build commands and write them to a file for later analysis.
/// - **Semantic only**: Read previously captured build commands from a file and analyze them.
/// - **Combined**: Capture build commands and analyze them in real-time.
///
/// Internally, this enum distinguishes between:
/// - `Intercept`: Modes that execute build commands while capturing events (intercept-only and combined)
/// - `Replay`: Modes that process previously captured events (semantic-only)
///
/// The distinction between writing raw events vs. performing semantic analysis
/// is handled by the consumer configuration, not the mode itself.
pub enum Mode {
    Intercept(execution::Interceptor, args::BuildCommand),
    Replay(execution::Replayer),
}

impl Mode {
    /// Configure the application mode based on the command line arguments and the configuration.
    ///
    /// Here we are checking if the command line arguments and configuration are valid.
    /// If the arguments are valid, we create the appropriate mode instance.
    /// If that is not the case, we try to return a useful error message.
    pub fn configure(
        context: context::Context,
        args: args::Arguments,
        config: config::Main,
    ) -> Result<Self, ConfigurationError> {
        match args.mode {
            args::Mode::Intercept { input, output } => {
                log::debug!("Mode: intercept build and write events");

                let (producer, address) =
                    CollectorOnTcp::new().map_err(ConfigurationError::CollectorCreation)?;

                let build = environment::BuildEnvironment::create(
                    &context,
                    &config.intercept,
                    &config.compilers,
                    address,
                )
                .map_err(ConfigurationError::ExecutorCreation)?;

                let consumer = impls::RawEventWriter::create(&output.path)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let intercept = execution::Interceptor::new(
                    Arc::new(impls::TcpEventProducer::create(producer)),
                    Box::new(consumer),
                    Box::new(impls::BuildExecutor::create(build)),
                );

                Ok(Self::Intercept(intercept, input))
            }
            args::Mode::Semantic { input, output } => {
                log::debug!("Mode: replay events and semantic analysis");

                let source = impls::RawEventReader::create(&input.path)?;
                let consumer = impls::SemanticEventWriter::create(output, &config)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let replayer = execution::Replayer::new(Box::new(source), Box::new(consumer));

                Ok(Self::Replay(replayer))
            }
            args::Mode::Combined { input, output } => {
                log::debug!("Mode: intercept build and semantic analysis");

                let (producer, address) =
                    CollectorOnTcp::new().map_err(ConfigurationError::CollectorCreation)?;

                let build = environment::BuildEnvironment::create(
                    &context,
                    &config.intercept,
                    &config.compilers,
                    address,
                )
                .map_err(ConfigurationError::ExecutorCreation)?;

                let consumer = impls::SemanticEventWriter::create(output, &config)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let intercept = execution::Interceptor::new(
                    Arc::new(impls::TcpEventProducer::create(producer)),
                    Box::new(consumer),
                    Box::new(impls::BuildExecutor::create(build)),
                );

                Ok(Self::Intercept(intercept, input))
            }
        }
    }

    /// Runs the application mode.
    ///
    /// This executes the build command in intercept mode or reads the event file in replay mode.
    /// All errors returned are runtime errors that occur after valid arguments and configuration
    /// have been provided.
    pub fn run(self) -> ExitCode {
        let status = match self {
            Self::Intercept(interceptor, command) => interceptor.run(command),
            Self::Replay(semantic) => semantic.run(),
        };
        status.unwrap_or_else(|error| {
            log::error!("Bear: {error}");
            ExitCode::FAILURE
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("Failed to create collector: {0}")]
    CollectorCreation(std::io::Error),
    #[error("Failed to create executor: {0}")]
    ExecutorCreation(environment::ConfigurationError),
    #[error("Failed to create consumer: {0}")]
    ConsumerCreation(output::WriterCreationError),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

mod impls {
    use super::execution;
    use super::ConfigurationError;
    use crate::args::BuildCommand;
    use crate::intercept::environment;
    use crate::intercept::reporter::ReporterError;
    use crate::intercept::supervise::SuperviseError;
    use crate::intercept::tcp::CollectorOnTcp;
    use crate::output::{
        ExecutionEventDatabase, SerializationFormat, WriterCreationError, WriterError,
    };
    use crate::{args, config, intercept, output, semantic};
    use crossbeam_channel::{Receiver, Sender};
    use std::process::ExitStatus;
    use std::{fs, io};

    pub(super) struct TcpEventProducer {
        source: CollectorOnTcp,
    }

    impl TcpEventProducer {
        pub(super) fn create(source: CollectorOnTcp) -> Self {
            Self { source }
        }
    }

    impl execution::Producer for TcpEventProducer {
        fn produce(&self, destination: Sender<intercept::Event>) -> Result<(), ReporterError> {
            for event in self.source.events() {
                match event {
                    Ok(event) => {
                        log::debug!("Forwarding event: {event:?}");
                        if let Err(error) = destination.send(event) {
                            log::error!("Failed to forward event: {error}");
                        }
                    }
                    Err(error) => {
                        log::warn!("Failed to receive event: {error}");
                    }
                }
            }

            Ok(())
        }
    }

    impl execution::Cancellable for TcpEventProducer {
        fn cancel(&self) -> Result<(), ReporterError> {
            self.source.shutdown()
        }
    }

    impl execution::CancellableProducer for TcpEventProducer {}

    /// Represents an event file reader to be event source.
    ///
    /// The event file is written by the interceptor mode and contains unprocessed
    /// events that can be later processed by the semantic analysis pipeline.
    pub(super) struct RawEventReader {
        path: std::path::PathBuf,
    }

    impl RawEventReader {
        /// Create a new raw event reader.
        ///
        /// This reader will read the intercepted events from a file in a raw format.
        pub(super) fn create(path: &std::path::Path) -> Result<Self, ConfigurationError> {
            if !path.exists() || !path.is_file() {
                return Err(ConfigurationError::InvalidConfiguration(format!(
                    "Event file not found: {path:?}"
                )));
            }

            Ok(Self {
                path: path.to_path_buf(),
            })
        }
    }

    impl execution::Producer for RawEventReader {
        /// Opens the event file and reads the events while dispatching them to
        /// the destination channel. Errors are logged and ignored.
        fn produce(&self, destination: Sender<intercept::Event>) -> Result<(), ReporterError> {
            let source = fs::File::open(&self.path)
                .map(io::BufReader::new)
                .map_err(ReporterError::Network)?;

            let events = ExecutionEventDatabase::read_and_ignore(source, |error| {
                log::warn!("Event file reading issue: {error:?}");
            });

            for event in events {
                if let Err(error) = destination.send(event) {
                    log::error!("Failed to forward event: {error}");
                }
            }

            Ok(())
        }
    }

    /// Represents a raw event writer to be used as a consumer.
    ///
    /// The raw event writer will write the intercepted events as they are observed
    /// without any transformation. This can be later replayed to analyze the build.
    pub(super) struct RawEventWriter {
        path: std::path::PathBuf,
        destination: io::BufWriter<fs::File>,
    }

    impl RawEventWriter {
        /// Create a new raw event writer.
        ///
        /// This writer will write the intercepted events to a file in a raw format.
        pub(super) fn create(path: &std::path::Path) -> Result<Self, WriterCreationError> {
            let destination = fs::File::create(path)
                .map(io::BufWriter::new)
                .map_err(|err| WriterCreationError::Io(path.to_path_buf(), err))?;

            Ok(Self {
                path: path.to_path_buf(),
                destination,
            })
        }
    }

    impl execution::Consumer for RawEventWriter {
        /// Using existing file format, write the intercepted events to the output file.
        fn consume(self: Box<Self>, events: Receiver<intercept::Event>) -> Result<(), WriterError> {
            ExecutionEventDatabase::write(self.destination, events.into_iter())
                .map_err(|err| WriterError::Io(self.path.clone(), err))
        }
    }

    /// Represents a semantic event writer as a consumer.
    ///
    /// The output of this writer is a semantic analysis of the build commands
    /// that were intercepted. It uses the semantic interpreter to transform the
    /// intercepted events into semantic events and writes them to the output file
    /// in the specified format.
    pub(super) struct SemanticEventWriter {
        interpreter: Box<dyn semantic::Interpreter>,
        writer: output::OutputWriter,
    }

    impl SemanticEventWriter {
        /// Create a new semantic analysis pipeline based on the output configuration.
        ///
        /// The `output` argument contains the configuration for the output file location,
        /// while the `config` argument contains the configuration for the semantic analysis
        /// and clang compilation database formatting.
        pub(super) fn create(
            output: args::BuildSemantic,
            config: &config::Main,
        ) -> Result<Self, WriterCreationError> {
            let interpreter = semantic::interpreters::create(config)
                .map_err(|err| WriterCreationError::Configuration(err.to_string()))?;
            let writer = output::OutputWriter::try_from((&output, config))?;

            Ok(Self {
                interpreter: Box::new(interpreter),
                writer,
            })
        }
    }

    impl execution::Consumer for SemanticEventWriter {
        /// Consume the intercepted events, and transform them into semantic events,
        /// and write them into the target file (with the right format).
        fn consume(self: Box<Self>, events: Receiver<intercept::Event>) -> Result<(), WriterError> {
            // Transform and log the events to semantics.
            let semantics = events
                .into_iter()
                .flat_map(|event| self.interpreter.recognize(&event.execution));

            // Consume the entries and write them to the output file.
            self.writer.write(semantics)?;

            Ok(())
        }
    }

    pub(super) struct BuildExecutor {
        environment: environment::BuildEnvironment,
    }

    impl BuildExecutor {
        /// Create a new build executor with the given environment.
        pub(super) fn create(environment: environment::BuildEnvironment) -> Self {
            Self { environment }
        }
    }

    impl execution::Executor for BuildExecutor {
        /// Execute the build command in the given environment.
        ///
        /// This will run the build command and return the exit code.
        fn run(&self, command: BuildCommand) -> Result<ExitStatus, SuperviseError> {
            self.environment.run_build(command)
        }
    }
}

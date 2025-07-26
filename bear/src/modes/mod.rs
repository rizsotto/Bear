// SPDX-License-Identifier: GPL-3.0-or-later

mod execution;

use crate::intercept::environment::BuildEnvironment;
use crate::intercept::tcp::CollectorOnTcp;
use crate::{args, config, output};
use std::process::ExitCode;
use std::sync::Arc;

/// Represent the modes the application can run in.
///
/// To the user the modes are:
/// - intercept only: capture build commands and write them to a file.
/// - semantic only: read build commands from a file and analyze them.
/// - combined: capture build commands and analyze them in one go.
///
/// This representation of the mode is based on if we are intercepting the build commands
/// or if we are replaying them. If we are analyzing the build events or just writing them
/// to a file is not relevant for the mode itself, but rather for the configuration.
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
        args: args::Arguments,
        config: config::Main,
    ) -> Result<Self, ConfigurationError> {
        match args.mode {
            args::Mode::Intercept { input, output } => {
                log::debug!("Mode: intercept build and write events");

                let (producer, address) =
                    CollectorOnTcp::new().map_err(ConfigurationError::CollectorCreation)?;

                let build = BuildEnvironment::create(&config.intercept, address)
                    .map_err(ConfigurationError::ExecutorCreation)?;

                let consumer = impls::RawEventWriter::create(&output.file_name)
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

                let source = impls::RawEventReader::create(&input.file_name)?;
                let consumer = impls::SemanticEventWriter::create(output, &config)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let replayer = execution::Replayer::new(Box::new(source), Box::new(consumer));

                Ok(Self::Replay(replayer))
            }
            args::Mode::Combined { input, output } => {
                log::debug!("Mode: intercept build and semantic analysis");

                let (producer, address) =
                    CollectorOnTcp::new().map_err(ConfigurationError::CollectorCreation)?;

                let build = BuildEnvironment::create(&config.intercept, address)
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

    /// It actually runs the application mode.
    ///
    /// This is when the build command is executed in the intercept mode or
    /// when the event file is read in the replay mode. These errors are all
    /// run-time errors, the user were passing valid arguments and configurations.
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
    ExecutorCreation(std::io::Error),
    #[error("Failed to create consumer: {0}")]
    ConsumerCreation(output::WriterCreationError),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

mod impls {
    use super::execution;
    use super::ConfigurationError;
    use crate::args::BuildCommand;
    use crate::intercept::supervise::SuperviseError;
    use crate::intercept::tcp::{CollectorOnTcp, ReceivingError};
    use crate::intercept::{environment, supervise};
    use crate::output::{ExecutionEventDatabase, FileFormat, FormatError, WriterCreationError};
    use crate::{args, config, intercept, output, semantic};
    use crossbeam_channel::{Receiver, Sender};
    use std::process::ExitStatus;
    use std::{fs, io, path};

    pub(super) struct TcpEventProducer {
        source: CollectorOnTcp,
    }

    impl TcpEventProducer {
        pub(super) fn create(source: CollectorOnTcp) -> Self {
            Self { source }
        }
    }

    impl execution::Producer<intercept::Event, ReceivingError> for TcpEventProducer {
        fn produce(&self, destination: Sender<intercept::Event>) -> Result<(), ReceivingError> {
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

    impl execution::Cancellable<ReceivingError> for TcpEventProducer {
        fn cancel(&self) -> Result<(), ReceivingError> {
            self.source.shutdown()
        }
    }

    impl execution::CancellableProducer<intercept::Event, ReceivingError> for TcpEventProducer {}

    /// Represents an event file reader to be event source.
    ///
    /// The event file is written by the interceptor mode and contains unprocessed
    /// events that can be later processed by the semantic analysis pipeline.
    pub(super) struct RawEventReader {
        file_name: path::PathBuf,
    }

    impl RawEventReader {
        /// Create a new raw event reader.
        ///
        /// This reader will read the intercepted events from a file in a raw format.
        pub(super) fn create(file_name: &str) -> Result<Self, ConfigurationError> {
            let file_path = path::PathBuf::from(file_name);
            if !file_path.exists() || !file_path.is_file() {
                return Err(ConfigurationError::InvalidConfiguration(format!(
                    "Event file not found: {file_name}"
                )));
            }

            Ok(Self {
                file_name: file_path,
            })
        }
    }

    impl execution::Producer<intercept::Event, ReceivingError> for RawEventReader {
        /// Opens the event file and reads the events while dispatching them to
        /// the destination channel. Errors are logged and ignored.
        fn produce(&self, destination: Sender<intercept::Event>) -> Result<(), ReceivingError> {
            let source = fs::File::open(&self.file_name)
                .map(io::BufReader::new)
                .map_err(ReceivingError::Network)?;

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
        destination: io::BufWriter<fs::File>,
    }

    impl RawEventWriter {
        /// Create a new raw event writer.
        ///
        /// This writer will write the intercepted events to a file in a raw format.
        pub(super) fn create(file_name: &str) -> Result<Self, WriterCreationError> {
            let destination = fs::File::create(file_name)
                .map(io::BufWriter::new)
                .map_err(WriterCreationError::Io)?;

            Ok(Self { destination })
        }
    }

    impl execution::Consumer<intercept::Event, FormatError> for RawEventWriter {
        /// Using existing file format, write the intercepted events to the output file.
        fn consume(self: Box<Self>, events: Receiver<intercept::Event>) -> Result<(), FormatError> {
            ExecutionEventDatabase::write(self.destination, events.into_iter())
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
        /// and the output format.
        pub(super) fn create(
            output: args::BuildSemantic,
            config: &config::Main,
        ) -> Result<Self, WriterCreationError> {
            let interpreter = Box::new(semantic::interpreters::create(config));
            let writer = output::OutputWriter::try_from((&output, &config.output))?;

            Ok(Self {
                interpreter,
                writer,
            })
        }
    }

    impl execution::Consumer<intercept::Event, FormatError> for SemanticEventWriter {
        /// Consume the intercepted events, and transform them into semantic events,
        /// and write them into the target file (with the right format).
        fn consume(self: Box<Self>, events: Receiver<intercept::Event>) -> Result<(), FormatError> {
            // Transform and log the events to semantics.
            let semantics = events
                .into_iter()
                .inspect(|event| {
                    log::debug!("Processing event: {event:?}");
                })
                .flat_map(|event| self.interpreter.recognize(&event.execution))
                .inspect(|semantic| {
                    log::debug!("Recognized semantic: {semantic:?}");
                });

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

    impl execution::Executor<SuperviseError> for BuildExecutor {
        /// Execute the build command in the given environment.
        ///
        /// This will run the build command and return the exit code.
        fn run(&self, command: BuildCommand) -> Result<ExitStatus, SuperviseError> {
            self.environment.run_build(command)
        }
    }
}

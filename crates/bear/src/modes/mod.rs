// SPDX-License-Identifier: GPL-3.0-or-later

//! # Execution Modes
//!
//! This module provides the core execution patterns for Bear's operation modes.
//! It defines traits and implementations for the producer-consumer pattern
//! used throughout the application.

mod execution;

use crate::environment;
use crate::semantic::interpreters::compilers::compiler_recognition::CompilerRecognizer;
use crate::{args, config, output};
use intercept_supervisor::CollectorOnTcp;
use intercept_supervisor::context;
use std::process::ExitCode;
use std::sync::Arc;

/// Returns true when `path` is the `-` sentinel meaning standard input or
/// standard output, depending on context.
///
/// `semantic --input -` reads the event stream from stdin (see
/// `docs/requirements/interception-events-format.md`). Modes that run the
/// build never accept `-` for output: the build's own stdout shares the
/// stream and would corrupt it.
fn is_stdio(path: &std::path::Path) -> bool {
    path.as_os_str() == "-"
}

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

                let recognizer = CompilerRecognizer::new();
                let build = environment::BuildEnvironment::create(
                    &context,
                    &config.intercept,
                    &config.compilers,
                    address,
                    |path| recognizer.recognize(path).is_some(),
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
                let consumer =
                    impls::SemanticEventWriter::create(output, &config, context.confstr_path.clone())
                        .map_err(ConfigurationError::ConsumerCreation)?;

                let replayer = execution::Replayer::new(Box::new(source), Box::new(consumer));

                Ok(Self::Replay(replayer))
            }
            args::Mode::Combined { input, output } => {
                log::debug!("Mode: intercept build and semantic analysis");

                if is_stdio(&output.path) {
                    return Err(ConfigurationError::InvalidConfiguration(
                        "cannot write the compilation database to stdout while running the build: \
                         it would mix with build output; write to a file, or split into \
                         `bear intercept` then `bear semantic`"
                            .to_string(),
                    ));
                }

                let (producer, address) =
                    CollectorOnTcp::new().map_err(ConfigurationError::CollectorCreation)?;

                let recognizer = CompilerRecognizer::new();
                let build = environment::BuildEnvironment::create(
                    &context,
                    &config.intercept,
                    &config.compilers,
                    address,
                    |path| recognizer.recognize(path).is_some(),
                )
                .map_err(ConfigurationError::ExecutorCreation)?;

                let consumer =
                    impls::SemanticEventWriter::create(output, &config, context.confstr_path.clone())
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
            log::error!("{error}");
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
    use super::ConfigurationError;
    use super::execution;
    use crate::args::BuildCommand;
    use crate::environment;
    use crate::output::{ExecutionEventDatabase, SerializationFormat, WriterCreationError, WriterError};
    use crate::{args, config, output, semantic};
    use crossbeam_channel::{Receiver, Sender};
    use intercept::reporter::ReporterError;
    use intercept_supervisor::CollectorOnTcp;
    use intercept_supervisor::SuperviseError;
    use std::process::ExitStatus;
    use std::sync::Arc;
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
        fn produce(&self, destination: Sender<intercept::Execution>) -> Result<(), ReporterError> {
            for execution in self.source.executions() {
                match execution {
                    Ok(execution) => {
                        // A disconnected destination means the consumer finished
                        // (normal shutdown) or failed (already logged upstream).
                        // Break out quietly rather than spamming an error line
                        // per remaining intercepted execution.
                        if destination.send(execution).is_err() {
                            log::debug!("Consumer channel closed; stopping execution forwarding");
                            break;
                        }
                    }
                    Err(error) => {
                        log::warn!("Failed to receive execution: {error}");
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
        /// This reader will read the intercepted events from a file in a raw format,
        /// or from standard input when the path is the `-` sentinel.
        pub(super) fn create(path: &std::path::Path) -> Result<Self, ConfigurationError> {
            if !super::is_stdio(path) && (!path.exists() || !path.is_file()) {
                return Err(ConfigurationError::InvalidConfiguration(format!(
                    "Event file not found: {path:?}"
                )));
            }

            Ok(Self { path: path.to_path_buf() })
        }

        /// Reads events from `source` and dispatches them to `destination`.
        /// Shared by the file and stdin code paths in `produce` so both keep
        /// identical parsing and error-reporting behaviour.
        fn forward(
            source: impl io::Read,
            destination: Sender<intercept::Execution>,
        ) -> Result<(), ReporterError> {
            let executions = ExecutionEventDatabase::read_and_ignore(source, |error| {
                log::warn!("Event file reading issue: {error:?}");
            });

            for execution in executions {
                if destination.send(execution).is_err() {
                    log::debug!("Consumer channel closed; stopping execution forwarding");
                    break;
                }
            }

            Ok(())
        }
    }

    impl execution::Producer for RawEventReader {
        /// Opens the event source (a file, or standard input when the path is
        /// `-`) and reads the executions while dispatching them to the
        /// destination channel. Errors are logged and ignored.
        fn produce(&self, destination: Sender<intercept::Execution>) -> Result<(), ReporterError> {
            if super::is_stdio(&self.path) {
                let stdin = io::stdin();
                let source = stdin.lock();
                return Self::forward(source, destination);
            }

            let source =
                fs::File::open(&self.path).map(io::BufReader::new).map_err(ReporterError::Network)?;

            Self::forward(source, destination)
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
        /// Rejects the `-` sentinel: interception runs the build, and the
        /// build's own stdout shares that stream, so a non-atomic write here
        /// could split a JSON line and corrupt the output.
        pub(super) fn create(path: &std::path::Path) -> Result<Self, WriterCreationError> {
            if super::is_stdio(path) {
                return Err(WriterCreationError::Configuration(
                    "cannot write events to stdout: interception shares stdout with the build \
                     and would corrupt the stream; write to a file instead"
                        .to_string(),
                ));
            }

            let destination = fs::File::create(path)
                .map(io::BufWriter::new)
                .map_err(|err| WriterCreationError::Io(path.to_path_buf(), err))?;

            Ok(Self { path: path.to_path_buf(), destination })
        }
    }

    impl execution::Consumer for RawEventWriter {
        /// Using existing file format, write the intercepted executions to the output file.
        fn consume(self: Box<Self>, executions: Receiver<intercept::Execution>) -> Result<(), WriterError> {
            ExecutionEventDatabase::write(self.destination, executions.into_iter())
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
            confstr_path: String,
        ) -> Result<Self, WriterCreationError> {
            let interpreter = semantic::interpreters::create(config, confstr_path);
            let writer = output::OutputWriter::try_from((&output, config))?;

            Ok(Self { interpreter: Box::new(interpreter), writer })
        }
    }

    impl execution::Consumer for SemanticEventWriter {
        /// Consume the intercepted executions, transform them into semantic events,
        /// and write them into the target file (with the right format).
        fn consume(self: Box<Self>, executions: Receiver<intercept::Execution>) -> Result<(), WriterError> {
            let stats = Arc::clone(self.writer.statistics());

            let semantics =
                executions.into_iter().filter_map(|execution| match self.interpreter.recognize(execution) {
                    semantic::RecognizeResult::Recognized(cmd) => {
                        stats.semantic_commands_received.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        Some(cmd)
                    }
                    semantic::RecognizeResult::Ignored(_) => {
                        stats.semantic_commands_received.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        None
                    }
                    semantic::RecognizeResult::NotRecognized(_) => None,
                });

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

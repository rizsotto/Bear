// SPDX-License-Identifier: GPL-3.0-or-later

//! # Execution Modes
//!
//! This module provides the core execution patterns for Bear's operation modes.
//! It defines traits and implementations for the producer-consumer pattern
//! used throughout the application.

mod execution;

use crate::args::is_stdio;
use crate::environment;
use crate::{args, config, output};
use intercept_supervisor::CollectorOnTcp;
use intercept_supervisor::context;
use semantic::SpellingError;
use semantic::{CompilerHints, CompilerRecognizer};
use std::process::ExitCode;
use std::sync::Arc;

/// Represents the application execution modes.
///
/// Bear supports four user-facing modes:
/// - **Intercept only**: Capture build commands and write them to a file for later analysis.
/// - **Semantic only**: Read previously captured build commands from a file and analyze them.
/// - **Combined**: Capture build commands and analyze them in real-time.
/// - **Parse shell text**: Interpret shell command text and produce the compilation database.
///
/// Internally, this enum distinguishes between:
/// - `Intercept`: Modes that execute build commands while capturing events (intercept-only and combined)
/// - `Replay`: Modes that process an existing input without running a build (semantic-only and parse-sh)
///
/// The distinction between the input sources and output formats is handled by
/// the producer and consumer configuration, not the mode itself.
pub enum Mode {
    Intercept(execution::Interceptor, args::BuildCommand),
    Replay(execution::Replayer),
    /// Print the compilers Bear recognizes, then exit. Consults no config
    /// and reads no input -- handled like parse-sh, before config load.
    PrintCompilers,
}

impl Mode {
    /// Configure the application mode from the command line arguments.
    ///
    /// For the modes that consume it, this loads the configuration (from
    /// `--config`, a default-location file, or built-in defaults), checks that
    /// the argument/configuration combination is valid, and builds the matching
    /// mode instance -- returning a useful error otherwise.
    pub fn configure(context: context::Context, args: args::Arguments) -> Result<Self, ConfigurationError> {
        let args::Arguments { config: config_path, mode } = args;

        // print-compilers runs without configuration: it only lists the
        // built-in recognition tables, so it loads none.
        if let args::Mode::PrintCompilers = mode {
            log::debug!("Mode: print recognized compilers");
            return Ok(Self::PrintCompilers);
        }

        let config = config::Loader::load(&context, &config_path)
            .map_err(|error| ConfigurationError::ConfigLoad(Box::new(error)))?;
        log::info!("{config}");

        // Resolve every configured `as:` spelling once, here, so a typo fails
        // at startup in every mode that reads configuration -- before a build
        // starts. This is also the only place that sees both the config types
        // and the semantic types, so it owns the conversion between them.
        let hints = compiler_hints(&config.compilers)?;

        match mode {
            args::Mode::Intercept { input, output } => {
                log::debug!("Mode: intercept build and write events");

                let (producer, address) =
                    CollectorOnTcp::new().map_err(ConfigurationError::CollectorCreation)?;

                let recognizer = CompilerRecognizer::new_with_hints(hints);
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
                let consumer = impls::SemanticEventWriter::create(output, &config, hints)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let replayer = execution::Replayer::new(Box::new(source), Box::new(consumer));

                Ok(Self::Replay(replayer))
            }
            args::Mode::ParseSh { input, output } => {
                log::debug!("Mode: parse shell text and semantic analysis");

                // When the caller left the directory override unset, fall
                // back to Bear's invocation directory so the producer always
                // has a concrete path to interpret from.
                let directory = input.directory.unwrap_or_else(|| context.current_directory.clone());
                let producer =
                    impls::ShellScriptReader::create(&input.path, directory, context.environment.clone())?;
                let consumer = impls::SemanticEventWriter::create(output, &config, hints)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let replayer = execution::Replayer::new(Box::new(producer), Box::new(consumer));

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

                let recognizer = CompilerRecognizer::new_with_hints(hints.clone());
                let build = environment::BuildEnvironment::create(
                    &context,
                    &config.intercept,
                    &config.compilers,
                    address,
                    |path| recognizer.recognize(path).is_some(),
                )
                .map_err(ConfigurationError::ExecutorCreation)?;

                let consumer = impls::SemanticEventWriter::create(output, &config, hints)
                    .map_err(ConfigurationError::ConsumerCreation)?;

                let intercept = execution::Interceptor::new(
                    Arc::new(impls::TcpEventProducer::create(producer)),
                    Box::new(consumer),
                    Box::new(impls::BuildExecutor::create(build)),
                );

                Ok(Self::Intercept(intercept, input))
            }
            // print-compilers is handled above, before configuration loads.
            args::Mode::PrintCompilers => unreachable!("print-compilers handled above"),
        }
    }

    /// Runs the application mode.
    ///
    /// This executes the build command in intercept mode, or reads the input
    /// source (event file or shell text) in replay mode. All errors returned
    /// are runtime errors that occur after valid arguments and configuration
    /// have been provided.
    pub fn run(self) -> ExitCode {
        let status = match self {
            Self::Intercept(interceptor, command) => interceptor.run(command),
            Self::Replay(replayer) => replayer.run(),
            Self::PrintCompilers => {
                print!(
                    "Bear {} recognizes the following compilers:\n\n{}",
                    env!("CARGO_PKG_VERSION"),
                    semantic::print_compilers()
                );
                return ExitCode::SUCCESS;
            }
        };
        status.unwrap_or_else(|error| {
            log::error!("{error}");
            ExitCode::FAILURE
        })
    }
}

/// Converts the configured compilers into the semantic hint table.
///
/// Every entry has its `as:` spelling resolved, including the ones marked
/// `ignore: true`: they contribute no hint, but a typo in one still fails the
/// run, the way it did when the configuration schema validated the spelling
/// itself. An entry without `as:` is classified by the builder.
fn compiler_hints(compilers: &[config::Compiler]) -> Result<CompilerHints, ConfigurationError> {
    let mut hints = CompilerHints::new();
    for compiler in compilers {
        let spelling = compiler.as_.as_deref();
        let resolved = if compiler.ignore {
            CompilerHints::check(spelling)
        } else {
            hints.add(&compiler.path, spelling)
        };
        resolved.map_err(|source| ConfigurationError::CompilerSpelling {
            path: compiler.path.display().to_string(),
            source,
        })?;
    }
    Ok(hints)
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
    #[error("Failed to load configuration: {0}")]
    ConfigLoad(Box<config::ConfigError>),
    /// Names the offending entry: serde used to point at the file and line,
    /// and the path is the equivalent locator now that the schema layer no
    /// longer resolves the spelling.
    #[error("Invalid compiler configuration for '{path}': {source}")]
    CompilerSpelling {
        path: String,
        #[source]
        source: SpellingError,
    },
}

mod impls {
    use super::ConfigurationError;
    use super::execution;
    use super::execution::DynError;
    use crate::args::BuildCommand;
    use crate::environment;
    use crate::output::{ExecutionEventDatabase, SerializationFormat, WriterCreationError, WriterError};
    use crate::{args, config, output, parse_sh};
    use crossbeam_channel::{Receiver, Sender};
    use intercept_supervisor::CollectorOnTcp;
    use intercept_supervisor::SuperviseError;
    use semantic::CompilerHints;
    use std::collections::HashMap;
    use std::io::IsTerminal;
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use std::sync::Arc;
    use std::{fs, io};
    use thiserror::Error;

    pub(super) struct TcpEventProducer {
        source: CollectorOnTcp,
    }

    impl TcpEventProducer {
        pub(super) fn create(source: CollectorOnTcp) -> Self {
            Self { source }
        }
    }

    impl execution::Producer for TcpEventProducer {
        fn produce(&self, destination: Sender<intercept::Execution>) -> Result<(), DynError> {
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
        fn cancel(&self) -> Result<(), DynError> {
            self.source.shutdown().map_err(Into::into)
        }
    }

    impl execution::CancellableProducer for TcpEventProducer {}

    /// Errors raised while reading and replaying a raw event file.
    #[derive(Debug, Error)]
    enum ReplayReadError {
        #[error("failed to open event file {0:?}: {1}")]
        Open(std::path::PathBuf, io::Error),
        #[error("every one of {0} event line(s) was rejected; no events to analyze")]
        AllRejected(usize),
    }

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
        /// or from standard input when the path is the `-` sentinel (the default:
        /// `semantic` is a filter and reads standard input unless a file is named).
        pub(super) fn create(path: &std::path::Path) -> Result<Self, ConfigurationError> {
            if super::is_stdio(path) {
                // A filter blocking on an interactive terminal is almost
                // always a missing producer in front of the pipe; say so
                // instead of waiting silently.
                if io::stdin().is_terminal() {
                    log::warn!(
                        "reading events from standard input (a terminal); \
                         pipe a producer in, or name an event file with --input"
                    );
                }
            } else if !path.exists() || !path.is_file() {
                return Err(ConfigurationError::InvalidConfiguration(format!(
                    "Event file not found: {path:?}"
                )));
            }

            Ok(Self { path: path.to_path_buf() })
        }
    }

    impl execution::Producer for RawEventReader {
        /// Opens the event source (a file, or standard input when the path is
        /// `-`) and reads it through `ExecutionEventDatabase::read`, the
        /// single line-numbering, resilient JSON-Lines parser (see
        /// `docs/requirements/interception-events-format.md`): a malformed
        /// line does not stop parsing of the remaining lines, and its
        /// `Display` already carries its physical line number.
        ///
        /// Tallies non-blank lines seen (`nonempty`) against accepted ones;
        /// if every non-empty line was rejected, the run fails so the caller
        /// does not exit successfully having analyzed nothing. An entirely
        /// empty stream stays a success (an empty database is a valid
        /// output) but is reported on stderr, since it is almost always a
        /// plumbing mistake.
        fn produce(&self, destination: Sender<intercept::Execution>) -> Result<(), DynError> {
            let source: Box<dyn io::Read> = if super::is_stdio(&self.path) {
                Box::new(io::stdin().lock())
            } else {
                Box::new(
                    fs::File::open(&self.path)
                        .map_err(|err| ReplayReadError::Open(self.path.clone(), err))?,
                )
            };

            let mut nonempty = 0usize;
            let mut accepted = 0usize;

            for result in ExecutionEventDatabase::read(source) {
                nonempty += 1;
                match result {
                    Ok(execution) => {
                        accepted += 1;
                        if destination.send(execution).is_err() {
                            log::debug!("Consumer channel closed; stopping execution forwarding");
                            break;
                        }
                    }
                    Err(error) => {
                        log::warn!("event stream: {error}");
                    }
                }
            }

            if nonempty == 0 {
                log::warn!("event stream: no events found in input");
            } else if accepted == 0 {
                return Err(ReplayReadError::AllRejected(nonempty).into());
            }

            Ok(())
        }
    }

    /// Errors raised while reading and interpreting shell command text.
    #[derive(Debug, Error)]
    enum ShellScriptReadError {
        #[error("failed to read shell text from standard input: {0}")]
        ReadStdin(io::Error),
        #[error("failed to read shell text file {0}: {1}")]
        ReadFile(std::path::PathBuf, io::Error),
        #[error("every non-empty line was skipped; no commands parsed (see warnings above)")]
        AllSkipped,
    }

    /// Represents a shell text reader to be an execution source.
    ///
    /// Streams the `parse_sh` tokenizer/parser pipeline over shell command
    /// text (e.g. a `make -n` capture) and produces the recognized commands
    /// as executions, as if `bear intercept` had observed them; the semantic
    /// stage downstream turns them into the compilation database.
    pub(super) struct ShellScriptReader {
        path: std::path::PathBuf,
        working_dir: std::path::PathBuf,
        environment: HashMap<String, String>,
    }

    impl ShellScriptReader {
        /// Create a new shell text reader.
        ///
        /// This reader will read the shell text from a file, or from standard
        /// input when the path is the `-` sentinel. The `working_dir` is the
        /// already-resolved initial working directory the parsed commands are
        /// interpreted from, and `environment` is the startup environment the
        /// parsed commands are interpreted against (captured once in `Context`,
        /// so `produce` performs no ambient environment I/O).
        pub(super) fn create(
            path: &std::path::Path,
            working_dir: std::path::PathBuf,
            environment: HashMap<String, String>,
        ) -> Result<Self, ConfigurationError> {
            if !super::is_stdio(path) && (!path.exists() || !path.is_file()) {
                return Err(ConfigurationError::InvalidConfiguration(format!(
                    "Shell text file not found: {path:?}"
                )));
            }

            Ok(Self { path: path.to_path_buf(), working_dir, environment })
        }
    }

    impl execution::Producer for ShellScriptReader {
        /// Streams the input (a file, or standard input when the path is
        /// `-`) through the parser and sends each recognized command down
        /// the channel as its line completes; memory stays bounded by the
        /// longest logical line. Skipped lines are logged as warnings;
        /// when every non-empty line was skipped, the run fails so the
        /// caller does not exit successfully having emitted nothing.
        fn produce(&self, destination: Sender<intercept::Execution>) -> Result<(), DynError> {
            let input: Box<dyn io::BufRead> = if super::is_stdio(&self.path) {
                Box::new(io::stdin().lock())
            } else {
                let file = fs::File::open(&self.path)
                    .map_err(|error| ShellScriptReadError::ReadFile(self.path.clone(), error))?;
                Box::new(io::BufReader::new(file))
            };

            let context = parse_sh::Context {
                working_dir: self.working_dir.clone(),
                environment: self.environment.clone(),
            };

            let mut emitted = 0usize;
            let mut skipped = 0usize;
            for event in parse_sh::parse(input, &context) {
                match event {
                    Err(error) => {
                        return Err(if super::is_stdio(&self.path) {
                            ShellScriptReadError::ReadStdin(error).into()
                        } else {
                            ShellScriptReadError::ReadFile(self.path.clone(), error).into()
                        });
                    }
                    Ok(parse_sh::Event::Skipped(line)) => {
                        log::warn!("line {}: skipped ({})", line.line, line.reason);
                        skipped += 1;
                    }
                    Ok(parse_sh::Event::Execution(execution)) => {
                        emitted += 1;
                        // Trim the environment to the build-relevant subset,
                        // matching the shape `bear intercept` emits (see
                        // `intercept::Execution::trim`).
                        if destination.send(execution.trim()).is_err() {
                            log::debug!("Consumer channel closed; stopping execution forwarding");
                            break;
                        }
                    }
                }
            }

            if skipped > 0 {
                log::warn!("parse-sh: {emitted} command(s) parsed, {skipped} line(s) skipped");
            } else {
                log::info!("parse-sh: {emitted} command(s) parsed, {skipped} line(s) skipped");
            }

            if emitted == 0 {
                if skipped > 0 {
                    return Err(ShellScriptReadError::AllSkipped.into());
                }
                log::warn!("parse-sh: no commands found in input");
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
        destination: Box<dyn io::Write + Send>,
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

            Self::create_file(path)
        }

        fn create_file(path: &std::path::Path) -> Result<Self, WriterCreationError> {
            let destination = fs::File::create(path)
                .map(io::BufWriter::new)
                .map_err(|err| WriterCreationError::Io(path.to_path_buf(), err))?;

            Ok(Self { path: path.to_path_buf(), destination: Box::new(destination) })
        }
    }

    impl execution::Consumer for RawEventWriter {
        /// Using existing file format, write the intercepted executions to the output file.
        fn consume(self: Box<Self>, executions: Receiver<intercept::Execution>) -> Result<(), DynError> {
            ExecutionEventDatabase::write(self.destination, executions.into_iter())
                .map_err(|err| WriterError::Io(self.path.clone(), err))?;
            Ok(())
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
        /// and clang compilation database formatting. The `hints` were resolved once in
        /// [`super::Mode::configure`].
        pub(super) fn create(
            output: args::BuildSemantic,
            config: &config::Main,
            hints: CompilerHints,
        ) -> Result<Self, WriterCreationError> {
            let ignored: Vec<PathBuf> = config
                .compilers
                .iter()
                .filter(|compiler| compiler.ignore)
                .map(|compiler| compiler.path.clone())
                .collect();

            let interpreter = semantic::interpreters::create(
                hints,
                ignored,
                config.format.arguments.from_response_files,
                config.format.arguments.from_environment,
            );
            let writer = output::OutputWriter::try_from((&output, config))?;

            Ok(Self { interpreter: Box::new(interpreter), writer })
        }
    }

    impl execution::Consumer for SemanticEventWriter {
        /// Consume the intercepted executions, transform them into semantic events,
        /// and write them into the target file (with the right format).
        fn consume(self: Box<Self>, executions: Receiver<intercept::Execution>) -> Result<(), DynError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    mod fixture {
        use super::*;
        use tempfile::TempDir;

        /// A configuration file on disk, plus the executable it names, so the
        /// compiler-path validation the loader performs passes and the run
        /// reaches the `as:` resolution.
        pub(super) struct ConfiguredRun {
            _root: TempDir,
            config_file: PathBuf,
            compiler: PathBuf,
            current_directory: PathBuf,
        }

        impl ConfiguredRun {
            /// Writes a config whose single compiler entry carries `spelling`
            /// and the given `ignore` flag.
            pub(super) fn with_compiler(spelling: &str, ignore: bool) -> Self {
                let root = tempfile::tempdir().expect("tempdir");
                let compiler = root.path().join("my-compiler");
                std::fs::write(&compiler, "#!/bin/sh\n").expect("write compiler");

                let config_file = root.path().join("bear.yml");
                std::fs::write(
                    &config_file,
                    format!(
                        "schema: \"4.2\"\ncompilers:\n  - path: {}\n    as: \"{}\"\n    ignore: {}\n",
                        compiler.display(),
                        spelling,
                        ignore
                    ),
                )
                .expect("write config");

                let current_directory = root.path().to_path_buf();
                Self { _root: root, config_file, compiler, current_directory }
            }

            /// The configured compiler's path, as the diagnostic should name it.
            pub(super) fn compiler_path(&self) -> String {
                self.compiler.display().to_string()
            }

            pub(super) fn context(&self) -> context::Context {
                context::Context {
                    current_executable: PathBuf::from("/usr/bin/bear"),
                    current_directory: self.current_directory.clone(),
                    environment: HashMap::new(),
                    preload_supported: false,
                    confstr_path: String::new(),
                }
            }

            /// Semantic mode reading standard input: the mode itself performs
            /// no work at configure time, so the outcome is the `as:` verdict.
            pub(super) fn arguments(&self) -> args::Arguments {
                args::Arguments {
                    config: Some(self.config_file.display().to_string()),
                    mode: args::Mode::Semantic {
                        input: args::BuildEvents { path: PathBuf::from("-") },
                        output: args::BuildSemantic { path: PathBuf::from("-"), append: false },
                    },
                }
            }
        }
    }

    /// A misspelled `as:` fails at startup, before any input is read. The
    /// message names the offending entry and still lists every accepted
    /// spelling. Ignored entries are held to the same standard: they
    /// contribute no hint, but a typo in one is still a configuration error.
    #[test]
    fn configure_rejects_an_unknown_compiler_as_spelling() {
        let cases = [("an active entry", false), ("an ignored entry", true)];

        for (case, ignore) in cases {
            let fixture = fixture::ConfiguredRun::with_compiler("invalid_compiler_type", ignore);

            let sut =
                Mode::configure(fixture.context(), fixture.arguments()).err().map(|error| error.to_string());

            let Some(message) = sut else {
                panic!("case: {case}, expected the configuration to be rejected");
            };
            assert!(message.contains(&fixture.compiler_path()), "case: {case}, message: {message}");
            assert!(message.contains("unknown compiler id"), "case: {case}, message: {message}");
            assert!(message.contains("invalid_compiler_type"), "case: {case}, message: {message}");
            assert!(message.contains("wrapper"), "case: {case}, message: {message}");
            assert!(message.contains("gcc"), "case: {case}, message: {message}");
        }
    }

    #[test]
    fn configure_accepts_a_known_compiler_as_spelling() {
        let cases = [("an active entry", false), ("an ignored entry", true)];

        for (case, ignore) in cases {
            let fixture = fixture::ConfiguredRun::with_compiler("gcc", ignore);

            let sut = Mode::configure(fixture.context(), fixture.arguments());

            assert!(sut.is_ok(), "case: {case}, expected the configuration to be accepted");
        }
    }

    /// An ignored entry resolves its spelling but contributes no hint, so
    /// recognition of that path falls back to the regex layer. Neither name
    /// matches any recognition pattern, so a verdict can only come from a hint.
    #[test]
    fn ignored_entries_contribute_no_hint() {
        let compilers = vec![
            config::Compiler {
                path: PathBuf::from("/opt/toolchain/quiet-tool"),
                as_: Some("clang".to_string()),
                ignore: true,
            },
            config::Compiler {
                path: PathBuf::from("/opt/toolchain/loud-tool"),
                as_: Some("clang".to_string()),
                ignore: false,
            },
        ];

        let sut = CompilerRecognizer::new_with_hints(compiler_hints(&compilers).unwrap());

        assert_eq!(sut.recognize(Path::new("/opt/toolchain/quiet-tool")), None);
        assert!(sut.recognize(Path::new("/opt/toolchain/loud-tool")).is_some());
    }
}

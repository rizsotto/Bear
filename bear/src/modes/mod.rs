// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::collector::{BuildInterceptor, ReceivingError};
use crate::intercept::Event;
use crate::output::{ExecutionEventDatabase, FileFormat};
use crate::semantic;
use crate::{args, config, output};
use anyhow::Context;
use std::io::BufReader;
use std::process::ExitCode;
use std::sync::mpsc::Receiver;
use std::{fs, io, path};

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
    Intercept(Intercept),
    Replay(Replay),
}

impl Mode {
    /// Configure the application mode based on the command line arguments and the configuration.
    ///
    /// Here we are checking if the command line arguments and configuration are valid.
    /// If the arguments are valid, we create the appropriate mode instance.
    /// If that is not the case, we try to return a useful error message.
    pub fn configure(args: args::Arguments, config: config::Main) -> anyhow::Result<Self> {
        match args.mode {
            args::Mode::Intercept { input, output } => {
                log::debug!("Mode: intercept build and write events");
                Intercept::configure_to_write(input, output, config).map(Self::Intercept)
            }
            args::Mode::Semantic { input, output } => {
                log::debug!("Mode: replay events and semantic analysis");
                Replay::configure_to_analyse(input, output, config).map(Self::Replay)
            }
            args::Mode::Combined { input, output } => {
                log::debug!("Mode: intercept build and semantic analysis");
                Intercept::configure_to_analyse(input, output, config).map(Self::Intercept)
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
            Self::Intercept(intercept) => intercept.run(),
            Self::Replay(semantic) => semantic.run(),
        };
        status.unwrap_or_else(|error| {
            log::error!("Bear: {error}");
            ExitCode::FAILURE
        })
    }
}

/// This represents the build interception mode.
///
/// Captures the build command and the event processing mechanism as a single value.
pub struct Intercept {
    command: args::BuildCommand,
    interceptor: BuildInterceptor,
}

impl Intercept {
    /// Configure the intercept mode to write the build events to a file.
    fn configure_to_write(
        command: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let file_name = path::PathBuf::from(output.file_name);
        let output_file = fs::File::create(file_name.as_path())
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {file_name:?}"))?;

        let consumer = move |candidates: Receiver<Result<Event, ReceivingError>>| {
            let events = candidates
                .into_iter()
                .filter_map(Self::filter_received_event);
            ExecutionEventDatabase::write(output_file, events).map_err(anyhow::Error::from)
        };

        let interceptor = BuildInterceptor::create(config, consumer)?;

        Ok(Self {
            command,
            interceptor,
        })
    }

    /// Configure the intercept mode to analyze the build events and write the analysis results
    /// into a file, based on the provided output configuration.
    fn configure_to_analyse(
        command: args::BuildCommand,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let analyzer = SemanticAnalysisPipeline::create(output, &config)?;
        let consumer = move |candidates: Receiver<Result<Event, ReceivingError>>| {
            let events = candidates
                .into_iter()
                .filter_map(Self::filter_received_event);
            analyzer.consume(events)
        };

        let interceptor = BuildInterceptor::create(config, consumer)?;

        Ok(Self {
            command,
            interceptor,
        })
    }

    /// Run the build command in the intercept environment.
    ///
    /// The exit code is based on the result of the build command.
    fn run(self) -> anyhow::Result<ExitCode> {
        self.interceptor.run_build(self.command)
    }

    /// Filter the failed events from the receiver.
    fn filter_received_event(candidate: Result<Event, ReceivingError>) -> Option<Event> {
        match candidate {
            Ok(event) => Some(event),
            Err(err) => {
                log::error!("Receiving event during build interception failed: {err:?}");
                None
            }
        }
    }
}

/// This represents the replay mode, when the build events are read from a file.
pub struct Replay {
    source: BufReader<fs::File>,
    analyzer: SemanticAnalysisPipeline,
}

impl Replay {
    /// Configure the replay mode to analyze the build events from a file.
    fn configure_to_analyse(
        input: args::BuildEvents,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let event_file_name = path::PathBuf::from(input.file_name);
        let event_file = fs::File::open(event_file_name.as_path())
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {event_file_name:?}"))?;

        let semantic = SemanticAnalysisPipeline::create(output, &config)?;

        Ok(Self {
            source: event_file,
            analyzer: semantic,
        })
    }

    /// Run the semantic mode by reading the event file and analyzing the events.
    ///
    /// The exit code is based on the result of the output writer.
    fn run(self) -> anyhow::Result<ExitCode> {
        let events = ExecutionEventDatabase::read_and_ignore(self.source, |error| {
            log::warn!("Event file reading issue: {error:?}")
        });

        self.analyzer.consume(events).map(|_| ExitCode::SUCCESS)
    }
}

/// Represents the semantic analysis pipeline.
///
/// This is a set of complex operations that are performed on the build events.
/// But the event source is not relevant for the semantic analysis itself.
struct SemanticAnalysisPipeline {
    interpreter: Box<dyn semantic::Interpreter>,
    writer: output::OutputWriter,
}

impl SemanticAnalysisPipeline {
    /// Create a new semantic analysis pipeline based on the output configuration.
    ///
    /// The `output` argument contains the configuration for the output file location,
    /// while the `config` argument contains the configuration for the semantic analysis
    /// and the output format.
    fn create(output: args::BuildSemantic, config: &config::Main) -> anyhow::Result<Self> {
        let interpreter = Box::new(semantic::interpreters::create(config));
        let writer = output::OutputWriter::try_from((&output, &config.output))?;

        Ok(Self {
            interpreter,
            writer,
        })
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis.
    fn consume(self, events: impl IntoIterator<Item = Event>) -> anyhow::Result<()> {
        // Set up the pipeline of compilation database entries.
        let semantics = events.into_iter().flat_map(|event| {
            // FIXME: add logging for this step
            self.interpreter.recognize(&event.execution)
        });
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        self.writer.write(semantics)?;

        Ok(())
    }
}

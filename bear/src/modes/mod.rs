// SPDX-License-Identifier: GPL-3.0-or-later

pub mod intercept;
pub mod semantic;

use crate::modes::intercept::BuildInterceptor;
use crate::modes::semantic::SemanticAnalysisPipeline;
use crate::output::formats::{ExecutionEventDatabase, FileFormat};
use crate::{args, config};
use anyhow::Context;
use std::io::BufReader;
use std::process::ExitCode;
use std::{fs, io, path};

/// The mode trait is used to run the application in different modes.
pub trait Mode {
    fn run(self) -> anyhow::Result<ExitCode>;
}

/// The intercept mode we are only capturing the build commands
/// and write it into the output file.
pub struct Intercept {
    command: args::BuildCommand,
    interceptor: BuildInterceptor,
}

impl Intercept {
    /// Create a new intercept mode instance.
    pub fn create(
        command: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let file_name = path::PathBuf::from(output.file_name);
        let output_file = fs::File::create(file_name.as_path())
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        let interceptor = BuildInterceptor::create(config, move |events| {
            ExecutionEventDatabase::write(output_file, events.iter()).map_err(anyhow::Error::from)
        })?;

        Ok(Self {
            command,
            interceptor,
        })
    }
}

impl Mode for Intercept {
    /// Run the intercept mode by setting up the collector service and
    /// the intercept environment. The build command is executed in the
    /// intercept environment.
    ///
    /// The exit code is based on the result of the build command.
    fn run(self) -> anyhow::Result<ExitCode> {
        self.interceptor.run_build_command(self.command)
    }
}

/// The semantic mode we are deduct the semantic meaning of the
/// executed commands from the build process.
pub struct Semantic {
    event_file: BufReader<fs::File>,
    semantic: SemanticAnalysisPipeline,
}

impl Semantic {
    pub fn create(
        input: args::BuildEvents,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let event_file_name = path::PathBuf::from(input.file_name);
        let event_file = fs::File::open(event_file_name.as_path())
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", event_file_name))?;

        let semantic = SemanticAnalysisPipeline::create(output, &config)?;

        Ok(Self {
            event_file,
            semantic,
        })
    }
}

impl Mode for Semantic {
    /// Run the semantic mode by reading the event file and analyzing the events.
    ///
    /// The exit code is based on the result of the output writer.
    fn run(self) -> anyhow::Result<ExitCode> {
        self.semantic
            .analyze_and_write(ExecutionEventDatabase::read_and_ignore(
                self.event_file,
                |error| {
                    log::warn!("Event file reading issue: {:?}", error);
                },
            ))
            .map(|_| ExitCode::SUCCESS)
    }
}

/// The all model is combining the intercept and semantic modes.
pub struct Combined {
    command: args::BuildCommand,
    interceptor: BuildInterceptor,
}

impl Combined {
    /// Create a new all mode instance.
    pub fn create(
        command: args::BuildCommand,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let semantic = SemanticAnalysisPipeline::create(output, &config)?;
        let interceptor =
            BuildInterceptor::create(config, move |events| semantic.analyze_and_write(events))?;

        Ok(Self {
            command,
            interceptor,
        })
    }
}

impl Mode for Combined {
    /// Run the all mode by setting up the collector service and the intercept environment.
    /// The build command is executed in the intercept environment. The collected events are
    /// then processed by the semantic recognition and transformation. The result is written
    /// to the output file.
    ///
    /// The exit code is based on the result of the build command.
    fn run(self) -> anyhow::Result<ExitCode> {
        self.interceptor.run_build_command(self.command)
    }
}

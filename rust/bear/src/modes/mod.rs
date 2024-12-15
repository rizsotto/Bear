// SPDX-License-Identifier: GPL-3.0-or-later

pub mod intercept;
pub mod semantic;

use crate::modes::intercept::BuildInterceptor;
use crate::modes::semantic::SemanticAnalysisPipeline;
use crate::output::event::{read, write};
use crate::{args, config};
use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::BufReader;
use std::process::ExitCode;

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
    pub fn from(
        command: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let file_name = output.file_name.as_str();
        let output_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_name)
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        let interceptor =
            BuildInterceptor::new(config, move |envelopes| write(output_file, envelopes))?;

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
    event_file: BufReader<File>,
    semantic: SemanticAnalysisPipeline,
}

impl Semantic {
    pub fn from(
        input: args::BuildEvents,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let file_name = input.file_name.as_str();
        let event_file = OpenOptions::new()
            .read(true)
            .open(file_name)
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        let semantic = SemanticAnalysisPipeline::from(output, &config)?;

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
            .analyze_and_write(read(self.event_file))
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
    pub fn from(
        command: args::BuildCommand,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let semantic = SemanticAnalysisPipeline::from(output, &config)?;
        let interceptor = BuildInterceptor::new(config, move |envelopes| {
            semantic.analyze_and_write(envelopes)
        })?;

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

// SPDX-License-Identifier: GPL-3.0-or-later

pub mod intercept;
pub mod recognition;
pub mod transformation;

use crate::input::EventFileReader;
use crate::ipc::Envelope;
use crate::output::OutputWriter;
use crate::{args, config};
use anyhow::Context;
use intercept::{CollectorService, InterceptEnvironment};
use recognition::Recognition;
use std::io::BufWriter;
use std::process::ExitCode;
use transformation::Transformation;

/// The mode trait is used to run the application in different modes.
pub trait Mode {
    fn run(self) -> anyhow::Result<ExitCode>;
}

/// The intercept mode we are only capturing the build commands
/// and write it into the output file.
pub struct Intercept {
    command: args::BuildCommand,
    output: args::BuildEvents,
    config: config::Intercept,
}

impl Intercept {
    /// Create a new intercept mode instance.
    pub fn new(
        command: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Intercept,
    ) -> Self {
        Self {
            command,
            output,
            config,
        }
    }

    /// Consume events and write them into the output file.
    fn write_to_file(
        output_file_name: String,
        envelopes: impl IntoIterator<Item = Envelope>,
    ) -> anyhow::Result<()> {
        let mut writer = std::fs::File::create(&output_file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create output file: {:?}", &output_file_name))?;
        for envelope in envelopes {
            envelope.write_into(&mut writer).with_context(|| {
                format!("Failed to write execution report: {:?}", &output_file_name)
            })?;
        }
        Ok(())
    }
}

impl Mode for Intercept {
    /// Run the intercept mode by setting up the collector service and
    /// the intercept environment. The build command is executed in the
    /// intercept environment.
    ///
    /// The exit code is based on the result of the build command.
    fn run(self) -> anyhow::Result<ExitCode> {
        let output_file_name = self.output.file_name.clone();
        let service = CollectorService::new(move |envelopes| {
            Self::write_to_file(output_file_name, envelopes)
        })
        .with_context(|| "Failed to create the ipc service")?;
        let environment = InterceptEnvironment::new(&self.config, service.address())
            .with_context(|| "Failed to create the ipc environment")?;

        let status = environment
            .execute_build_command(self.command)
            .with_context(|| "Failed to execute the build command")?;

        Ok(status)
    }
}

/// The semantic mode we are deduct the semantic meaning of the
/// executed commands from the build process.
pub struct Semantic {
    event_source: EventFileReader,
    semantic_recognition: Recognition,
    semantic_transform: Transformation,
    output_writer: OutputWriter,
}

impl Semantic {
    /// Create a new semantic mode instance.
    pub fn new(
        event_source: EventFileReader,
        semantic_recognition: Recognition,
        semantic_transform: Transformation,
        output_writer: OutputWriter,
    ) -> Self {
        Self {
            event_source,
            semantic_recognition,
            semantic_transform,
            output_writer,
        }
    }
}

impl Mode for Semantic {
    /// Run the semantic mode by generating the compilation database entries
    /// from the event source. The entries are then processed by the semantic
    /// recognition and transformation. The result is written to the output file.
    ///
    /// The exit code is based on the result of the output writer.
    fn run(self) -> anyhow::Result<ExitCode> {
        // Set up the pipeline of compilation database entries.
        let entries = self
            .event_source
            .generate()
            .flat_map(|execution| self.semantic_recognition.apply(execution))
            .flat_map(|semantic| self.semantic_transform.apply(semantic));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        match self.output_writer.run(entries) {
            Ok(_) => Ok(ExitCode::SUCCESS),
            Err(_) => Ok(ExitCode::FAILURE),
        }
    }
}

/// The all model is combining the intercept and semantic modes.
pub struct All {
    command: args::BuildCommand,
    intercept_config: config::Intercept,
    semantic_recognition: Recognition,
    semantic_transform: Transformation,
    output_writer: OutputWriter,
}

impl All {
    /// Create a new all mode instance.
    pub fn new(
        command: args::BuildCommand,
        intercept_config: config::Intercept,
        semantic_recognition: Recognition,
        semantic_transform: Transformation,
        output_writer: OutputWriter,
    ) -> Self {
        Self {
            command,
            intercept_config,
            semantic_recognition,
            semantic_transform,
            output_writer,
        }
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis. Same as the `Semantic` mode.
    fn consume_for_analysis(
        semantic_recognition: Recognition,
        semantic_transform: Transformation,
        output_writer: OutputWriter,
        envelopes: impl IntoIterator<Item = Envelope>,
    ) -> anyhow::Result<()> {
        let entries = envelopes
            .into_iter()
            .map(|envelope| envelope.event.execution)
            .flat_map(|execution| semantic_recognition.apply(execution))
            .flat_map(|semantic| semantic_transform.apply(semantic));

        output_writer.run(entries)
    }
}

impl Mode for All {
    /// Run the all mode by setting up the collector service and the intercept environment.
    /// The build command is executed in the intercept environment. The collected events are
    /// then processed by the semantic recognition and transformation. The result is written
    /// to the output file.
    ///
    /// The exit code is based on the result of the build command.
    fn run(self) -> anyhow::Result<ExitCode> {
        let semantic_recognition = self.semantic_recognition;
        let semantic_transform = self.semantic_transform;
        let output_writer = self.output_writer;
        let service = CollectorService::new(move |envelopes| {
            Self::consume_for_analysis(
                semantic_recognition,
                semantic_transform,
                output_writer,
                envelopes,
            )
        })
        .with_context(|| "Failed to create the ipc service")?;
        let environment = InterceptEnvironment::new(&self.intercept_config, service.address())
            .with_context(|| "Failed to create the ipc environment")?;

        let status = environment
            .execute_build_command(self.command)
            .with_context(|| "Failed to execute the build command")?;

        Ok(status)
    }
}

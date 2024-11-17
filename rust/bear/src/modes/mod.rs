// SPDX-License-Identifier: GPL-3.0-or-later

mod intercept;
pub mod recognition;
pub mod transformation;

use crate::input::EventFileReader;
use crate::intercept::Envelope;
use crate::output::OutputWriter;
use crate::{args, config};
use anyhow::Context;
use crossbeam_channel::Receiver;
use intercept::{InterceptEnvironment, InterceptService};
use recognition::Recognition;
use std::io::BufWriter;
use std::process::ExitCode;
use transformation::Transformation;

/// The mode trait is used to run the application in different modes.
pub trait Mode {
    fn run(self) -> anyhow::Result<ExitCode>;
}

/// The intercept mode we are only capturing the build commands.
pub struct Intercept {
    command: args::BuildCommand,
    output: args::BuildEvents,
    config: config::Intercept,
}

/// The semantic mode we are deduct the semantic meaning of the
/// executed commands from the build process.
pub struct Semantic {
    event_source: EventFileReader,
    semantic_recognition: Recognition,
    semantic_transform: Transformation,
    output_writer: OutputWriter,
}

/// The all model is combining the intercept and semantic modes.
pub struct All {
    input: args::BuildCommand,
    output: args::BuildSemantic,
    intercept_config: config::Intercept,
    output_config: config::Output,
}

impl Intercept {
    pub fn new(
        input: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Intercept,
    ) -> Self {
        Self {
            command: input,
            output,
            config,
        }
    }

    /// Write the envelopes into the output file.
    fn write_to_file(
        output_file_name: String,
        envelopes: Receiver<Envelope>,
    ) -> anyhow::Result<()> {
        let mut writer = std::fs::File::create(&output_file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create output file: {:?}", &output_file_name))?;
        for envelope in envelopes.iter() {
            envelope
                .write_into(&mut writer)
                .with_context(|| "Failed to write the envelope")?;
        }
        Ok(())
    }
}

impl Mode for Intercept {
    fn run(self) -> anyhow::Result<ExitCode> {
        // TODO: log failures with the right context
        let output_file_name = self.output.file_name.clone();
        let service = InterceptService::new(move |envelopes| {
            Self::write_to_file(output_file_name, envelopes)
        })
        .with_context(|| "Failed to create the intercept service")?;
        let environment = InterceptEnvironment::new(&self.config, service.address())
            .with_context(|| "Failed to create the intercept environment")?;

        let status = environment
            .execute_build_command(self.command)
            .with_context(|| "Failed to execute the build command")?;

        Ok(status)
    }
}

impl Semantic {
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

impl All {
    pub fn new(
        input: args::BuildCommand,
        output: args::BuildSemantic,
        intercept_config: config::Intercept,
        output_config: config::Output,
    ) -> Self {
        Self {
            input,
            output,
            intercept_config,
            output_config,
        }
    }
}

impl Mode for All {
    fn run(self) -> anyhow::Result<ExitCode> {
        // TODO: Implement the all mode.
        Ok(ExitCode::FAILURE)
    }
}

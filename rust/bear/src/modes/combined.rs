// SPDX-License-Identifier: GPL-3.0-or-later

use crate::ipc::Envelope;
use crate::modes::intercept::{CollectorService, InterceptEnvironment};
use crate::modes::semantic::Recognition;
use crate::modes::Mode;
use crate::output::OutputWriter;
use crate::semantic::transformation::Transformation;
use crate::semantic::Transform;
use crate::{args, config};
use anyhow::Context;
use std::process::ExitCode;

/// The all model is combining the intercept and semantic modes.
pub struct Combined {
    command: args::BuildCommand,
    intercept_config: config::Intercept,
    semantic_recognition: Recognition,
    semantic_transform: Transformation,
    output_writer: OutputWriter,
}

impl Combined {
    /// Create a new all mode instance.
    pub fn from(
        command: args::BuildCommand,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let semantic_recognition = Recognition::try_from(&config)?;
        let semantic_transform = Transformation::from(&config.output);
        let output_writer = OutputWriter::configure(&output, &config.output)?;
        let intercept_config = config.intercept;

        Ok(Self {
            command,
            intercept_config,
            semantic_recognition,
            semantic_transform,
            output_writer,
        })
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

impl Mode for Combined {
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

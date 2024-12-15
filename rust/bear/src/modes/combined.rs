// SPDX-License-Identifier: GPL-3.0-or-later

use crate::modes::intercept::{CollectorService, InterceptEnvironment};
use crate::modes::semantic::SemanticFromEnvelopes;
use crate::modes::Mode;
use crate::{args, config};
use anyhow::Context;
use std::process::ExitCode;

/// The all model is combining the intercept and semantic modes.
pub struct Combined {
    command: args::BuildCommand,
    intercept_config: config::Intercept,
    semantic: SemanticFromEnvelopes,
}

impl Combined {
    /// Create a new all mode instance.
    pub fn from(
        command: args::BuildCommand,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let semantic = SemanticFromEnvelopes::from(output, &config)?;
        let intercept_config = config.intercept;

        Ok(Self {
            command,
            intercept_config,
            semantic,
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
        let semantic = self.semantic;
        let service = CollectorService::new(move |envelopes| semantic.analyze_and_write(envelopes))
            .with_context(|| "Failed to create the ipc service")?;

        let status = InterceptEnvironment::new(&self.intercept_config, service.address())
            .with_context(|| "Failed to create the ipc environment")?
            .execute_build_command(self.command)
            .with_context(|| "Failed to execute the build command")?;

        Ok(status)
    }
}

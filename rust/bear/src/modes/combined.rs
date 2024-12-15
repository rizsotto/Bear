// SPDX-License-Identifier: GPL-3.0-or-later

use crate::modes::intercept::Interceptor;
use crate::modes::semantic::SemanticFromEnvelopes;
use crate::modes::Mode;
use crate::{args, config};
use std::process::ExitCode;

/// The all model is combining the intercept and semantic modes.
pub struct Combined {
    command: args::BuildCommand,
    interceptor: Interceptor,
}

impl Combined {
    /// Create a new all mode instance.
    pub fn from(
        command: args::BuildCommand,
        output: args::BuildSemantic,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        let semantic = SemanticFromEnvelopes::from(output, &config)?;
        let interceptor: Interceptor = Interceptor::new(config, move |envelopes| {
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

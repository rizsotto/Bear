// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::{CollectorService, Envelope, InterceptEnvironment};
use crate::{args, config};
use anyhow::Context;
use std::process::ExitCode;
use std::sync::mpsc::Receiver;

/// The build interceptor is responsible for capturing the build commands and
/// dispatching them to the consumer. The consumer is a function that processes
/// the intercepted command executions.
pub(super) struct BuildInterceptor {
    environment: InterceptEnvironment,
}

impl BuildInterceptor {
    /// Create a new process execution interceptor.
    pub(super) fn new<F>(config: config::Main, consumer: F) -> anyhow::Result<Self>
    where
        F: FnOnce(Receiver<Envelope>) -> anyhow::Result<()>,
        F: Send + 'static,
    {
        let service = CollectorService::new(consumer)
            .with_context(|| "Failed to create the intercept service")?;

        let environment = InterceptEnvironment::new(&config.intercept, service)
            .with_context(|| "Failed to create the intercept environment")?;

        Ok(Self { environment })
    }

    /// Run the build command in the intercept environment.
    pub(super) fn run_build_command(self, command: args::BuildCommand) -> anyhow::Result<ExitCode> {
        self.environment
            .execute_build_command(command)
            .with_context(|| "Failed to execute the build command")
    }
}

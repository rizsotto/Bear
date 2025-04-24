// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::{CollectorService, Event, InterceptEnvironment};
use crate::{args, config};
use anyhow::Context;
use std::process::ExitCode;
use std::sync::mpsc::Receiver;

/// The build interceptor is responsible for capturing the build commands and
/// dispatching them to the consumer. The consumer is a function that processes
/// the intercepted command executions.
pub(super) struct BuildInterceptor {
    environment: InterceptEnvironment,
    #[allow(dead_code)]
    service: CollectorService,
}

impl BuildInterceptor {
    /// Create a new process execution interceptor.
    pub(super) fn create<F>(config: config::Main, consumer: F) -> anyhow::Result<Self>
    where
        F: FnOnce(Receiver<Event>) -> anyhow::Result<()>,
        F: Send + 'static,
    {
        let service = CollectorService::create(consumer)
            .with_context(|| "Failed to create the intercept service")?;

        let environment = InterceptEnvironment::create(&config.intercept, &service)
            .with_context(|| "Failed to create the intercept environment")?;

        Ok(Self {
            environment,
            service,
        })
    }

    /// Run the build command in the intercept environment.
    pub(super) fn run_build_command(self, command: args::BuildCommand) -> anyhow::Result<ExitCode> {
        self.environment
            .execute_build_command(command)
            .with_context(|| "Failed to execute the build command")
    }
}

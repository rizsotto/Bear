// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::collector::{CollectorService, InterceptEnvironment, ReceivingError};
use crate::intercept::Event;
use crate::{args, config};
use anyhow::Context;
use std::process::ExitCode;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

/// The build interceptor is responsible for capturing the build commands and
/// dispatching them to the consumer. The consumer is a function that processes
/// the intercepted command executions.
pub(super) struct BuildInterceptor {
    environment: InterceptEnvironment,
    #[allow(dead_code)]
    service: CollectorService,
    writer_thread: Option<thread::JoinHandle<()>>,
}

impl BuildInterceptor {
    /// Create a new process execution interceptor with a closure consumer.
    pub(super) fn create<F>(config: config::Main, consumer: F) -> anyhow::Result<Self>
    where
        F: FnOnce(Receiver<Result<Event, ReceivingError>>) -> anyhow::Result<()> + Send + 'static,
    {
        let (sender, receiver) = channel::<Result<Event, ReceivingError>>();

        let writer_thread = thread::spawn(move || {
            if let Err(err) = consumer(receiver) {
                log::error!("Failed to process intercepted events: {err:?}");
            }
        });

        let (service, address) = CollectorService::create(sender)
            .with_context(|| "Failed to create the intercept service")?;

        let environment = InterceptEnvironment::create(&config.intercept, address)
            .with_context(|| "Failed to create the intercept environment")?;

        Ok(Self {
            environment,
            service,
            writer_thread: Some(writer_thread),
        })
    }

    /// Run the build command in the intercept environment.
    pub(super) fn run_build_command(self, command: args::BuildCommand) -> anyhow::Result<ExitCode> {
        let result = self
            .environment
            .execute_build_command(command)
            .with_context(|| "Failed to execute the build command")?;

        if let Some(thread) = self.writer_thread {
            if let Err(err) = thread.join() {
                log::error!("Failed to join the intercept writer thread: {err:?}");
            }
        }

        Ok(result)
    }
}

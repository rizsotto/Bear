// SPDX-License-Identifier: GPL-3.0-or-later

use super::Mode;
use crate::ipc::tcp::CollectorOnTcp;
use crate::ipc::{Collector, Envelope};
use crate::{args, config};
use anyhow::Context;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::{env, thread};

/// Declare the environment variables used by the intercept mode.
pub const KEY_DESTINATION: &str = "INTERCEPT_REPORTER_ADDRESS";
pub const KEY_PRELOAD_PATH: &str = "LD_PRELOAD";

/// The intercept mode we are only capturing the build commands
/// and write it into the output file.
pub struct Intercept {
    command: args::BuildCommand,
    output: args::BuildEvents,
    config: config::Intercept,
}

impl Intercept {
    /// Create a new intercept mode instance.
    pub fn from(
        command: args::BuildCommand,
        output: args::BuildEvents,
        config: config::Main,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            command,
            output,
            config: config.intercept,
        })
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
            serde_json::to_writer(&mut writer, &envelope).with_context(|| {
                format!("Failed to write execution report: {:?}", &output_file_name)
            })?;
            // TODO: add a newline character to separate the entries
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

/// The service is responsible for collecting the events from the supervised processes.
///
/// The service is implemented as TCP server that listens on a random port on the loopback
/// interface. The address of the service can be obtained by the `address` method.
///
/// The service is started in a separate thread to dispatch the events to the consumer.
/// The consumer is a function that receives the events from the service and processes them.
/// It also runs in a separate thread. The reason for having two threads is to avoid blocking
/// the main thread of the application and decouple the collection from the processing.
pub(super) struct CollectorService {
    collector: Arc<dyn Collector>,
    network_thread: Option<thread::JoinHandle<()>>,
    output_thread: Option<thread::JoinHandle<()>>,
}

impl CollectorService {
    /// Creates a new ipc service.
    ///
    /// The `consumer` is a function that receives the events and processes them.
    /// The function is executed in a separate thread.
    pub fn new<F>(consumer: F) -> anyhow::Result<Self>
    where
        F: FnOnce(Receiver<Envelope>) -> anyhow::Result<()>,
        F: Send + 'static,
    {
        let collector = CollectorOnTcp::new()?;
        let collector_arc = Arc::new(collector);
        let (sender, receiver) = channel();

        let collector_in_thread = collector_arc.clone();
        let collector_thread = thread::spawn(move || {
            let result = collector_in_thread.collect(sender);
            if let Err(e) = result {
                log::error!("Failed to collect events: {}", e);
            }
        });
        let output_thread = thread::spawn(move || {
            let result = consumer(receiver);
            if let Err(e) = result {
                log::error!("Failed to process events: {}", e);
            }
        });

        log::debug!("Collector service started at {}", collector_arc.address());
        Ok(CollectorService {
            collector: collector_arc,
            network_thread: Some(collector_thread),
            output_thread: Some(output_thread),
        })
    }

    /// Returns the address of the service.
    pub fn address(&self) -> String {
        self.collector.address()
    }
}

impl Drop for CollectorService {
    /// Shuts down the service.
    fn drop(&mut self) {
        // TODO: log the shutdown of the service and any errors
        self.collector.stop().expect("Failed to stop the collector");
        if let Some(thread) = self.network_thread.take() {
            thread.join().expect("Failed to join the collector thread");
        }
        if let Some(thread) = self.output_thread.take() {
            thread.join().expect("Failed to join the output thread");
        }
    }
}

/// The environment for the intercept mode.
///
/// Running the build command requires a specific environment. The environment we
/// need for intercepting the child processes is different for each intercept mode.
///
/// The `Wrapper` mode requires a temporary directory with the executables that will
/// be used to intercept the child processes. The executables are hard linked to the
/// temporary directory.
///
/// The `Preload` mode requires the path to the preload library that will be used to
/// intercept the child processes.
pub(super) enum InterceptEnvironment {
    Wrapper {
        bin_dir: tempfile::TempDir,
        address: String,
    },
    Preload {
        path: PathBuf,
        address: String,
    },
}

impl InterceptEnvironment {
    /// Creates a new intercept environment.
    ///
    /// The `config` is the intercept configuration that specifies the mode and the
    /// required parameters for the mode. The `address` is the address of the ipc
    /// service that will be used to collect the events.
    pub fn new(config: &config::Intercept, address: String) -> anyhow::Result<Self> {
        let result = match config {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                // Create a temporary directory and populate it with the executables.
                let bin_dir = tempfile::TempDir::with_prefix_in(directory, "bear-")?;
                for executable in executables {
                    std::fs::hard_link(executable, path)?;
                }
                InterceptEnvironment::Wrapper { bin_dir, address }
            }
            config::Intercept::Preload { path } => InterceptEnvironment::Preload {
                path: path.clone(),
                address,
            },
        };
        Ok(result)
    }

    /// Executes the build command in the intercept environment.
    ///
    /// The method is blocking and waits for the build command to finish.
    /// The method returns the exit code of the build command. Result failure
    /// indicates that the build command failed to start.
    pub fn execute_build_command(self, input: args::BuildCommand) -> anyhow::Result<ExitCode> {
        // TODO: record the execution of the build command

        let environment = self.environment();
        let mut child = Command::new(input.arguments[0].clone())
            .args(input.arguments[1..].iter())
            .envs(environment)
            .spawn()?;

        // TODO: forward signals to the child process
        let result = child.wait()?;

        // The exit code is not always available. When the process is killed by a signal,
        // the exit code is not available. In this case, we return the `FAILURE` exit code.
        let exit_code = result
            .code()
            .map(|code| ExitCode::from(code as u8))
            .unwrap_or(ExitCode::FAILURE);

        Ok(exit_code)
    }

    /// Returns the environment variables for the intercept environment.
    ///
    /// The environment variables are different for each intercept mode.
    /// It does not change the original environment variables, but creates
    /// the environment variables that are required for the ipc mode.
    fn environment(&self) -> Vec<(String, String)> {
        match self {
            InterceptEnvironment::Wrapper {
                bin_dir, address, ..
            } => {
                let path_original = env::var("PATH").unwrap_or_else(|_| String::new());
                let path_updated = InterceptEnvironment::insert_to_path(
                    &path_original,
                    Self::path_to_string(bin_dir.path()),
                );
                vec![
                    ("PATH".to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
            InterceptEnvironment::Preload { path, address, .. } => {
                let path_original = env::var(KEY_PRELOAD_PATH).unwrap_or_else(|_| String::new());
                let path_updated = InterceptEnvironment::insert_to_path(
                    &path_original,
                    Self::path_to_string(path),
                );
                vec![
                    (KEY_PRELOAD_PATH.to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
        }
    }

    /// Manipulate a `PATH` like environment value by inserting the `first` path into
    /// the original value. It removes the `first` path if it already exists in the
    /// original value. And it inserts the `first` path at the beginning of the value.
    fn insert_to_path(original: &str, first: String) -> String {
        let mut paths: Vec<_> = original.split(':').filter(|it| it != &first).collect();
        paths.insert(0, first.as_str());
        paths.join(":")
    }

    fn path_to_string(path: &Path) -> String {
        path.to_str().unwrap_or("").to_string()
    }
}

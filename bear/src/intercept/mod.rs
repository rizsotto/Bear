// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the intercept reporting and collecting functionality.
//!
//! When a command execution is intercepted, the interceptor sends the event to the collector.
//! This happens in two different processes, requiring a communication channel between these
//! processes.
//!
//! The module provides abstractions for the reporter and the collector. And it also defines
//! the data structures that are used to represent the events.

pub mod supervise;
mod tcp;

use crate::{args, config};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::{fmt, thread};
use supervise::supervise;

/// Declare the environment variables used by the intercept mode.
pub const KEY_DESTINATION: &str = "INTERCEPT_COLLECTOR_ADDRESS";
const KEY_PRELOAD_PATH: &str = "LD_PRELOAD";

/// Creates a new reporter instance.
///
/// This function is supposed to be called from another process than the collector.
/// Therefore, the reporter destination is passed as an environment variable.
/// The reporter destination is the address of the collector service.
pub fn create_reporter() -> anyhow::Result<Box<dyn Reporter>> {
    let reporter =
        // Get the reporter address from the environment variable
        std::env::var(KEY_DESTINATION)
        .with_context(|| format!("${} is missing from the environment", KEY_DESTINATION))
        // Create a new reporter
        .map(tcp::ReporterOnTcp::new)?;
    Ok(Box::new(reporter))
}

pub fn create_reporter_on_tcp(address: &str) -> Box<dyn Reporter> {
    let reporter = tcp::ReporterOnTcp::new(address.to_string());
    Box::new(reporter)
}

/// Represents the remote sink of supervised process events.
///
/// This allows the reporters to send events to a remote collector.
pub trait Reporter {
    fn report(&self, event: Event) -> Result<(), anyhow::Error>;
}

/// Represents the local sink of supervised process events.
///
/// The collector is responsible for collecting the events from the reporters.
///
/// To share the collector between threads, we use the `Arc` type to wrap the
/// collector. This way we can clone the collector and send it to other threads.
pub trait Collector {
    /// Returns the address of the collector.
    ///
    /// The address is in the format of `ip:port`.
    fn address(&self) -> String;

    /// Collects the events from the reporters.
    ///
    /// The events are sent to the given destination channel.
    ///
    /// The function returns when the collector is stopped. The collector is stopped
    /// when the `stop` method invoked (from another thread).
    fn collect(&self, destination: Sender<Event>) -> Result<(), anyhow::Error>;

    /// Stops the collector.
    fn stop(&self) -> Result<(), anyhow::Error>;
}

/// Represent a relevant life cycle event of a process.
///
/// In the current implementation, we only have one event, the `Started` event.
/// This event is sent when a process is started. It contains the process id
/// and the execution information.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event {
    pub pid: u32,
    pub execution: Execution,
}

impl Event {
    /// Creates a new event that is originated from the current process.
    pub fn new(execution: Execution) -> Self {
        let pid = std::process::id();
        Event { pid, execution }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Event pid={}, execution={}", self.pid, self.execution)
    }
}

/// Execution is a representation of a process execution.
///
/// It does not contain information about the outcome of the execution,
/// like the exit code or the duration of the execution. It only contains
/// the information that is necessary to reproduce the execution.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Execution {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
}

impl Execution {
    /// Capture the execution information of the current process.
    pub fn capture() -> anyhow::Result<Self> {
        let executable = std::env::current_exe()?;
        let arguments = std::env::args().collect();
        let working_dir = std::env::current_dir()?;
        let environment = std::env::vars().collect();

        Ok(Self {
            executable,
            arguments,
            working_dir,
            environment,
        })
    }

    pub fn with_executable(&self, executable: &Path) -> Self {
        let mut updated = self.clone();
        updated.executable = executable.to_path_buf();
        updated
    }

    pub fn with_environment(&self, environment: HashMap<String, String>) -> Self {
        let mut updated = self.clone();
        updated.environment = environment;
        updated
    }
}

impl TryFrom<args::BuildCommand> for Execution {
    type Error = anyhow::Error;

    /// Converts the `BuildCommand` to an `Execution` object.
    fn try_from(value: args::BuildCommand) -> Result<Self, Self::Error> {
        let executable = value
            .arguments
            .first()
            .ok_or_else(|| anyhow::anyhow!("No executable found"))?
            .clone()
            .into();
        let arguments = value.arguments.to_vec();
        let working_dir = std::env::current_dir()?;
        let environment = std::env::vars().collect();

        Ok(Self {
            executable,
            arguments,
            working_dir,
            environment,
        })
    }
}

impl From<Execution> for std::process::Command {
    fn from(val: Execution) -> Self {
        let mut command = std::process::Command::new(val.executable);
        command.args(val.arguments.iter().skip(1));
        command.envs(val.environment);
        command.current_dir(val.working_dir);
        command
    }
}

impl fmt::Display for Execution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Execution path={}, args=[{}]",
            self.executable.display(),
            self.arguments.join(",")
        )
    }
}

#[cfg(test)]
pub fn execution(
    executable: &str,
    arguments: Vec<&str>,
    working_dir: &str,
    environment: HashMap<&str, &str>,
) -> Execution {
    Execution {
        executable: PathBuf::from(executable),
        arguments: arguments.iter().map(|s| s.to_string()).collect(),
        working_dir: PathBuf::from(working_dir),
        environment: environment
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

#[cfg(test)]
pub fn event(
    pid: u32,
    executable: &str,
    arguments: Vec<&str>,
    working_dir: &str,
    environment: HashMap<&str, &str>,
) -> Event {
    Event {
        pid,
        execution: execution(executable, arguments, working_dir, environment),
    }
}

/// The service is responsible for collecting the events from the supervised processes.
///
/// The service is implemented as a TCP server that listens to on a random port on the loopback
/// interface. The address of the service can be obtained by the `address` method.
///
/// The service is started in a separate thread to dispatch the events to the consumer.
/// The consumer is a function that receives the events from the service and processes them.
/// It also runs in a separate thread. The reason for having two threads is to avoid blocking
/// the main thread of the application and decouple the collection from the processing.
pub struct CollectorService {
    collector: Arc<dyn Collector>,
    network_thread: Option<thread::JoinHandle<()>>,
    output_thread: Option<thread::JoinHandle<()>>,
}

impl CollectorService {
    /// Creates a new intercept service.
    ///
    /// The `consumer` is a function that receives the events and processes them.
    /// The function is executed in a separate thread.
    pub fn create<F>(consumer: F) -> anyhow::Result<Self>
    where
        F: FnOnce(Receiver<Event>) -> anyhow::Result<()>,
        F: Send + 'static,
    {
        let collector = tcp::CollectorOnTcp::create()?;
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
pub enum InterceptEnvironment {
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
    /// required parameters for the mode. The `collector` is the service to collect
    /// the execution events.
    pub fn create(
        config: &config::Intercept,
        collector: &CollectorService,
    ) -> anyhow::Result<Self> {
        // Validate the configuration.
        let valid_config = config.validate()?;

        let address = collector.address();
        let result = match &valid_config {
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
    pub fn execute_build_command(&self, input: args::BuildCommand) -> anyhow::Result<ExitCode> {
        // TODO: record the execution of the build command

        let child: Execution = TryInto::<Execution>::try_into(input)?
            .with_environment(self.environment().into_iter().collect());
        let exit_status = supervise(child)?;
        log::info!("Execution finished with status: {:?}", exit_status);

        // The exit code is not always available. When the process is killed by a signal,
        // the exit code is not available. In this case, we return the `FAILURE` exit code.
        let exit_code = exit_status
            .code()
            .map(|code| ExitCode::from(code as u8))
            .unwrap_or(ExitCode::FAILURE);

        Ok(exit_code)
    }

    /// Returns the environment variables for the intercept environment.
    ///
    /// The environment variables are different for each intercept mode.
    /// It does not change the original environment variables, but creates
    /// the environment variables that are required for the intercept mode.
    fn environment(&self) -> Vec<(String, String)> {
        match self {
            InterceptEnvironment::Wrapper {
                bin_dir, address, ..
            } => {
                let path_original = std::env::var("PATH").unwrap_or_else(|_| String::new());
                let path_updated = InterceptEnvironment::insert_to_path(
                    &path_original,
                    bin_dir.path().to_path_buf(),
                )
                .unwrap_or_else(|_| path_original.clone());
                vec![
                    ("PATH".to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
            InterceptEnvironment::Preload { path, address, .. } => {
                let path_original =
                    std::env::var(KEY_PRELOAD_PATH).unwrap_or_else(|_| String::new());
                let path_updated =
                    InterceptEnvironment::insert_to_path(&path_original, path.clone())
                        .unwrap_or_else(|_| path_original.clone());
                vec![
                    (KEY_PRELOAD_PATH.to_string(), path_updated),
                    (KEY_DESTINATION.to_string(), address.clone()),
                ]
            }
        }
    }

    /// Manipulate a `PATH`-like environment value by inserting the `first` path into
    /// the original value. It removes the `first` path if it already exists in the
    /// original value. And it inserts the `first` path at the beginning of the value.
    fn insert_to_path(original: &str, first: PathBuf) -> anyhow::Result<String> {
        let mut paths: Vec<_> = std::env::split_paths(original)
            .filter(|path| path != &first)
            .collect();
        paths.insert(0, first);
        std::env::join_paths(paths)
            .map(|os_string| os_string.into_string().unwrap_or_default())
            .map_err(|e| anyhow::anyhow!("Failed to join paths: {}", e))
    }
}

impl config::Intercept {
    /// Validate the configuration of the intercept mode.
    fn validate(&self) -> anyhow::Result<Self> {
        match self {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                if Self::is_empty_path(path) {
                    anyhow::bail!("The wrapper path cannot be empty.");
                }
                if Self::is_empty_path(directory) {
                    anyhow::bail!("The wrapper directory cannot be empty.");
                }
                for executable in executables {
                    if Self::is_empty_path(executable) {
                        anyhow::bail!("The executable path cannot be empty.");
                    }
                }
                Ok(self.clone())
            }
            config::Intercept::Preload { path } => {
                if Self::is_empty_path(path) {
                    anyhow::bail!("The preload library path cannot be empty.");
                }
                Ok(self.clone())
            }
        }
    }

    fn is_empty_path(path: &Path) -> bool {
        path.to_str().is_some_and(|p| p.is_empty())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_validate_intercept_wrapper_valid() {
        let sut = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from("/tmp"),
            executables: vec![PathBuf::from("/usr/bin/cc")],
        };
        assert!(sut.validate().is_ok());
    }

    #[test]
    fn test_validate_intercept_wrapper_empty_path() {
        let sut = config::Intercept::Wrapper {
            path: PathBuf::from(""),
            directory: PathBuf::from("/tmp"),
            executables: vec![PathBuf::from("/usr/bin/cc")],
        };
        assert!(sut.validate().is_err());
    }

    #[test]
    fn test_validate_intercept_wrapper_empty_directory() {
        let sut = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from(""),
            executables: vec![PathBuf::from("/usr/bin/cc")],
        };
        assert!(sut.validate().is_err());
    }

    #[test]
    fn test_validate_intercept_wrapper_empty_executables() {
        let sut = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from("/tmp"),
            executables: vec![
                PathBuf::from("/usr/bin/cc"),
                PathBuf::from("/usr/bin/c++"),
                PathBuf::from(""),
            ],
        };
        assert!(sut.validate().is_err());
    }

    #[test]
    fn test_validate_intercept_preload_valid() {
        let sut = config::Intercept::Preload {
            path: PathBuf::from("/usr/local/lib/libexec.so"),
        };
        assert!(sut.validate().is_ok());
    }

    #[test]
    fn test_validate_intercept_preload_empty_path() {
        let sut = config::Intercept::Preload {
            path: PathBuf::from(""),
        };
        assert!(sut.validate().is_err());
    }
}

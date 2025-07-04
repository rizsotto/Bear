// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::{supervise, tcp, Event, Execution, KEY_DESTINATION, KEY_PRELOAD_PATH};
use crate::{args, config};
use anyhow::Context;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use thiserror::Error;

/// Represents the local sink of supervised process events.
///
/// The collector is responsible for collecting the events from the reporters.
pub trait Collector: Send + Sync {
    /// Creates a new collector instance.
    ///
    /// The collector listens to a random port on the loopback interface.
    /// The returned address can be used to connect to the collector.
    fn create(
        destination: Sender<Result<Event, ReceivingError>>,
    ) -> Result<(Self, SocketAddr), CollectorError>
    where
        Self: Sized;

    /// Collects the events from the reporters.
    ///
    /// The events are sent to the given destination channel.
    ///
    /// The function returns when the collector is stopped or failed.
    /// To request the collector to stop collecting events, call the `stop` method.
    fn start(&self) -> Result<(), CollectorError>;

    /// Request the collector to stop collecting events.
    fn stop(&self) -> Result<(), CollectorError>;
}

/// Errors that can occur to set up the collector.
#[derive(Error, Debug)]
pub enum CollectorError {
    #[error("Collecting events failed with IO error: {0}")]
    Network(#[from] std::io::Error),
    #[error("Collecting events failed with internal IPC error: {0}")]
    Channel(String),
}

/// Errors that can occur in the collector.
#[derive(Error, Debug)]
pub enum ReceivingError {
    #[error("Receiving event failed with IO error: {0}")]
    Network(#[from] std::io::Error),
    #[error("Receiving event failed with serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
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
    collector_arc: Arc<dyn Collector>,
    // Declare the thread handle as option to allow take ownership and join it later.
    collector_thread: Option<thread::JoinHandle<()>>,
}

impl CollectorService {
    /// Creates a new intercept service.
    ///
    /// The `consumer` is a function that receives the events and processes them.
    /// The function is executed in a separate thread.
    pub fn create(
        destination: Sender<Result<Event, ReceivingError>>,
    ) -> Result<(Self, SocketAddr), CollectorError> {
        let (collector, address) = tcp::CollectorOnTcp::create(destination)?;
        let collector_arc = Arc::new(collector);

        let collector_in_thread = collector_arc.clone();
        let collector_thread = thread::spawn(move || {
            let result = collector_in_thread.start();
            if let Err(err) = result {
                log::error!("Failed to collect events: {err}");
            }
        });

        log::debug!("Collector service started at {address}");
        Ok((
            Self {
                collector_arc,
                collector_thread: Some(collector_thread),
            },
            address,
        ))
    }
}

impl Drop for CollectorService {
    /// Shuts down the service.
    fn drop(&mut self) {
        if let Err(err) = self.collector_arc.stop() {
            log::error!("Failed to stop the collector: {err}");
        }
        if let Some(handle) = self.collector_thread.take() {
            if let Err(err) = handle.join() {
                log::error!("Failed to join collector thread {err:?}");
            }
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
    // FIXME: the environment should be captured here.
    Wrapper {
        bin_dir: tempfile::TempDir,
        address: SocketAddr,
    },
    Preload {
        path: PathBuf,
        address: SocketAddr,
    },
}

impl InterceptEnvironment {
    /// Creates a new intercept environment.
    ///
    /// The `config` is the intercept configuration that specifies the mode and the
    /// required parameters for the mode. The `collector` is the service to collect
    /// the execution events.
    pub fn create(config: &config::Intercept, address: SocketAddr) -> Result<Self, InterceptError> {
        // Validate the configuration.
        let valid_config = config.validate()?;

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
    pub fn execute_build_command(
        &self,
        input: args::BuildCommand,
    ) -> Result<ExitCode, InterceptError> {
        // TODO: record the execution of the build command

        let child: Execution = Self::execution(input, self.environment())?;
        let exit_status = supervise::supervise(child)
            .map_err(|e| InterceptError::ProcessExecution(e.to_string()))?;
        log::info!("Execution finished with status: {exit_status:?}");

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
                    (KEY_DESTINATION.to_string(), address.to_string()),
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
                    (KEY_DESTINATION.to_string(), address.to_string()),
                ]
            }
        }
    }

    /// Manipulate a `PATH`-like environment value by inserting the `first` path into
    /// the original value. It removes the `first` path if it already exists in the
    /// original value. And it inserts the `first` path at the beginning of the value.
    fn insert_to_path(original: &str, first: PathBuf) -> Result<String, InterceptError> {
        let mut paths: Vec<_> = std::env::split_paths(original)
            .filter(|path| path != &first)
            .collect();
        paths.insert(0, first);
        std::env::join_paths(paths)
            .map(|os_string| os_string.into_string().unwrap_or_default())
            .map_err(InterceptError::from)
    }

    fn execution(
        input: args::BuildCommand,
        environment: Vec<(String, String)>,
    ) -> Result<Execution, InterceptError> {
        let executable = input
            .arguments
            .first()
            .ok_or(InterceptError::NoExecutable)?
            .clone()
            .into();
        let arguments = input.arguments.to_vec();
        let working_dir = std::env::current_dir().map_err(InterceptError::Io)?;
        let environment = environment.into_iter().collect::<HashMap<String, String>>();

        Ok(Execution {
            executable,
            arguments,
            working_dir,
            environment,
        })
    }
}

impl config::Intercept {
    /// Validate the configuration of the intercept mode.
    fn validate(&self) -> Result<Self, InterceptError> {
        match self {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                if Self::is_empty_path(path) {
                    return Err(InterceptError::ConfigValidation(
                        "The wrapper path cannot be empty.".to_string(),
                    ));
                }
                if Self::is_empty_path(directory) {
                    return Err(InterceptError::ConfigValidation(
                        "The wrapper directory cannot be empty.".to_string(),
                    ));
                }
                for executable in executables {
                    if Self::is_empty_path(executable) {
                        return Err(InterceptError::ConfigValidation(
                            "The executable path cannot be empty.".to_string(),
                        ));
                    }
                }
                Ok(self.clone())
            }
            config::Intercept::Preload { path } => {
                if Self::is_empty_path(path) {
                    return Err(InterceptError::ConfigValidation(
                        "The preload library path cannot be empty.".to_string(),
                    ));
                }
                Ok(self.clone())
            }
        }
    }

    fn is_empty_path(path: &Path) -> bool {
        path.to_str().is_some_and(|p| p.is_empty())
    }
}

/// Errors that can occur in the intercept environment and configuration.
// FIXME: this should be simplified
#[derive(Error, Debug)]
pub enum InterceptError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Process execution error: {0}")]
    ProcessExecution(String),
    #[error("Configuration validation error: {0}")]
    ConfigValidation(String),
    #[error("Path manipulation error: {0}")]
    Path(String),
    #[error("Thread join error")]
    ThreadJoin,
    #[error("No executable found in build command")]
    NoExecutable,
    #[error("Collector error: {0}")]
    Collector(#[from] CollectorError),
}

// FIXME: this should be removed when error is simplified
impl From<std::env::JoinPathsError> for InterceptError {
    fn from(err: std::env::JoinPathsError) -> Self {
        InterceptError::Path(format!("Failed to join paths: {err}"))
    }
}

/// The build interceptor is responsible for capturing the build commands and
/// dispatching them to the consumer. The consumer is a function that processes
/// the intercepted command executions.
pub(crate) struct BuildInterceptor {
    environment: InterceptEnvironment,
    #[allow(dead_code)]
    service: CollectorService,
    writer_thread: Option<thread::JoinHandle<()>>,
}

impl BuildInterceptor {
    /// Create a new process execution interceptor with a closure consumer.
    pub fn create<F>(config: config::Main, consumer: F) -> anyhow::Result<Self>
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
    pub fn run_build(self, command: args::BuildCommand) -> anyhow::Result<ExitCode> {
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

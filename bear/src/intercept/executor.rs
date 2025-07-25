// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::config;
use crate::intercept::supervise;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::ExitStatus;

/// A trait for executing build commands.
///
/// Executors are responsible for running the actual build process while
/// allowing command interception to occur. They manage the lifecycle of
/// the build command and report its exit status.
///
/// # Type Parameters
/// - `E`: The error type that can occur during execution
pub trait Executor<E> {
    /// Executes the given build command.
    ///
    /// This is a blocking operation that runs the build command to completion.
    /// During execution, the command and its subprocesses may be intercepted
    /// by Bear's interception mechanisms.
    ///
    /// # Arguments
    /// * `command` - The build command to execute
    ///
    /// # Returns
    /// * `Ok(ExitCode)` - The build completed with the given exit code
    /// * `Err(E)` - An error occurred during execution
    fn run(&self, _: BuildCommand) -> Result<ExitStatus, E>;
}

pub struct BuildExecutor {
    environment: HashMap<String, String>,
}

impl BuildExecutor {
    pub fn create(config: &config::Intercept, address: SocketAddr) -> Result<Self, std::io::Error> {
        todo!()
    }

    fn build(&self, val: BuildCommand) -> std::process::Command {
        let mut command = std::process::Command::new(val.arguments.first().unwrap());
        command.args(val.arguments.iter().skip(1));
        command.envs(self.environment.clone());
        command
    }
}

impl Executor<supervise::SuperviseError> for BuildExecutor {
    fn run(&self, build_command: BuildCommand) -> Result<ExitStatus, supervise::SuperviseError> {
        log::debug!("Running build command: {build_command:?}");
        let mut command = self.build(build_command);
        supervise::supervise(&mut command)
    }
}

// /// The environment for the intercept mode.
// ///
// /// Running the build command requires a specific environment. The environment we
// /// need for intercepting the child processes is different for each intercept mode.
// ///
// /// The `Wrapper` mode requires a temporary directory with the executables that will
// /// be used to intercept the child processes. The executables are hard linked to the
// /// temporary directory.
// ///
// /// The `Preload` mode requires the path to the preload library that will be used to
// /// intercept the child processes.
// pub enum InterceptEnvironment {
//     // FIXME: the environment should be captured here.
//     Wrapper {
//         bin_dir: tempfile::TempDir,
//         address: SocketAddr,
//     },
//     Preload {
//         path: PathBuf,
//         address: SocketAddr,
//     },
// }
//
// impl InterceptEnvironment {
//     /// Creates a new intercept environment.
//     ///
//     /// The `config` is the intercept configuration that specifies the mode and the
//     /// required parameters for the mode. The `collector` is the service to collect
//     /// the execution events.
//     pub fn create(config: &config::Intercept, address: SocketAddr) -> Result<Self, InterceptError> {
//         // Validate the configuration.
//         let valid_config = config.validate()?;
//
//         let result = match &valid_config {
//             config::Intercept::Wrapper {
//                 path,
//                 directory,
//                 executables,
//             } => {
//                 // Create a temporary directory and populate it with the executables.
//                 let bin_dir = tempfile::TempDir::with_prefix_in(directory, "bear-")?;
//                 for executable in executables {
//                     std::fs::hard_link(executable, path)?;
//                 }
//                 InterceptEnvironment::Wrapper { bin_dir, address }
//             }
//             config::Intercept::Preload { path } => InterceptEnvironment::Preload {
//                 path: path.clone(),
//                 address,
//             },
//         };
//         Ok(result)
//     }
//
//     /// Executes the build command in the intercept environment.
//     ///
//     /// The method is blocking and waits for the build command to finish.
//     /// The method returns the exit code of the build command. Result failure
//     /// indicates that the build command failed to start.
//     pub fn execute_build_command(
//         &self,
//         input: args::BuildCommand,
//     ) -> Result<ExitCode, InterceptError> {
//         // TODO: record the execution of the build command
//
//         let child: Execution = Self::execution(input, self.environment())?;
//         let exit_status = supervise::supervise_execution(child)
//             .map_err(|e| InterceptError::ProcessExecution(e.to_string()))?;
//         log::info!("Execution finished with status: {exit_status:?}");
//
//         // The exit code is not always available. When the process is killed by a signal,
//         // the exit code is not available. In this case, we return the `FAILURE` exit code.
//         let exit_code = exit_status
//             .code()
//             .map(|code| ExitCode::from(code as u8))
//             .unwrap_or(ExitCode::FAILURE);
//
//         Ok(exit_code)
//     }
//
//     /// Returns the environment variables for the intercept environment.
//     ///
//     /// The environment variables are different for each intercept mode.
//     /// It does not change the original environment variables, but creates
//     /// the environment variables that are required for the intercept mode.
//     fn environment(&self) -> Vec<(String, String)> {
//         match self {
//             InterceptEnvironment::Wrapper {
//                 bin_dir, address, ..
//             } => {
//                 let path_original = std::env::var("PATH").unwrap_or_else(|_| String::new());
//                 let path_updated = InterceptEnvironment::insert_to_path(
//                     &path_original,
//                     bin_dir.path().to_path_buf(),
//                 )
//                 .unwrap_or_else(|_| path_original.clone());
//                 vec![
//                     ("PATH".to_string(), path_updated),
//                     (KEY_DESTINATION.to_string(), address.to_string()),
//                 ]
//             }
//             InterceptEnvironment::Preload { path, address, .. } => {
//                 let path_original =
//                     std::env::var(KEY_PRELOAD_PATH).unwrap_or_else(|_| String::new());
//                 let path_updated =
//                     InterceptEnvironment::insert_to_path(&path_original, path.clone())
//                         .unwrap_or_else(|_| path_original.clone());
//                 vec![
//                     (KEY_PRELOAD_PATH.to_string(), path_updated),
//                     (KEY_DESTINATION.to_string(), address.to_string()),
//                 ]
//             }
//         }
//     }
//
//     /// Manipulate a `PATH`-like environment value by inserting the `first` path into
//     /// the original value. It removes the `first` path if it already exists in the
//     /// original value. And it inserts the `first` path at the beginning of the value.
//     fn insert_to_path(original: &str, first: PathBuf) -> Result<String, InterceptError> {
//         let mut paths: Vec<_> = std::env::split_paths(original)
//             .filter(|path| path != &first)
//             .collect();
//         paths.insert(0, first);
//         std::env::join_paths(paths)
//             .map(|os_string| os_string.into_string().unwrap_or_default())
//             .map_err(InterceptError::from)
//     }
//
//     fn execution(
//         input: args::BuildCommand,
//         environment: Vec<(String, String)>,
//     ) -> Result<Execution, InterceptError> {
//         let executable = input
//             .arguments
//             .first()
//             .ok_or(InterceptError::NoExecutable)?
//             .clone()
//             .into();
//         let arguments = input.arguments.to_vec();
//         let working_dir = std::env::current_dir().map_err(InterceptError::Io)?;
//         let environment = environment.into_iter().collect::<HashMap<String, String>>();
//
//         Ok(Execution {
//             executable,
//             arguments,
//             working_dir,
//             environment,
//         })
//     }
// }

// impl config::Intercept {
//     /// Validate the configuration of the intercept mode.
//     fn validate(&self) -> Result<Self, InterceptError> {
//         match self {
//             config::Intercept::Wrapper {
//                 path,
//                 directory,
//                 executables,
//             } => {
//                 if Self::is_empty_path(path) {
//                     return Err(InterceptError::ConfigValidation(
//                         "The wrapper path cannot be empty.".to_string(),
//                     ));
//                 }
//                 if Self::is_empty_path(directory) {
//                     return Err(InterceptError::ConfigValidation(
//                         "The wrapper directory cannot be empty.".to_string(),
//                     ));
//                 }
//                 for executable in executables {
//                     if Self::is_empty_path(executable) {
//                         return Err(InterceptError::ConfigValidation(
//                             "The executable path cannot be empty.".to_string(),
//                         ));
//                     }
//                 }
//                 Ok(self.clone())
//             }
//             config::Intercept::Preload { path } => {
//                 if Self::is_empty_path(path) {
//                     return Err(InterceptError::ConfigValidation(
//                         "The preload library path cannot be empty.".to_string(),
//                     ));
//                 }
//                 Ok(self.clone())
//             }
//         }
//     }
//
//     fn is_empty_path(path: &Path) -> bool {
//         path.to_str().is_some_and(|p| p.is_empty())
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//
//     #[test]
//     fn test_validate_intercept_wrapper_valid() {
//         let sut = config::Intercept::Wrapper {
//             path: PathBuf::from("/usr/bin/wrapper"),
//             directory: PathBuf::from("/tmp"),
//             executables: vec![PathBuf::from("/usr/bin/cc")],
//         };
//         assert!(sut.validate().is_ok());
//     }
//
//     #[test]
//     fn test_validate_intercept_wrapper_empty_path() {
//         let sut = config::Intercept::Wrapper {
//             path: PathBuf::from(""),
//             directory: PathBuf::from("/tmp"),
//             executables: vec![PathBuf::from("/usr/bin/cc")],
//         };
//         assert!(sut.validate().is_err());
//     }
//
//     #[test]
//     fn test_validate_intercept_wrapper_empty_directory() {
//         let sut = config::Intercept::Wrapper {
//             path: PathBuf::from("/usr/bin/wrapper"),
//             directory: PathBuf::from(""),
//             executables: vec![PathBuf::from("/usr/bin/cc")],
//         };
//         assert!(sut.validate().is_err());
//     }
//
//     #[test]
//     fn test_validate_intercept_wrapper_empty_executables() {
//         let sut = config::Intercept::Wrapper {
//             path: PathBuf::from("/usr/bin/wrapper"),
//             directory: PathBuf::from("/tmp"),
//             executables: vec![
//                 PathBuf::from("/usr/bin/cc"),
//                 PathBuf::from("/usr/bin/c++"),
//                 PathBuf::from(""),
//             ],
//         };
//         assert!(sut.validate().is_err());
//     }
//
//     #[test]
//     fn test_validate_intercept_preload_valid() {
//         let sut = config::Intercept::Preload {
//             path: PathBuf::from("/usr/local/lib/libexec.so"),
//         };
//         assert!(sut.validate().is_ok());
//     }
//
//     #[test]
//     fn test_validate_intercept_preload_empty_path() {
//         let sut = config::Intercept::Preload {
//             path: PathBuf::from(""),
//         };
//         assert!(sut.validate().is_err());
//     }
// }

// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::config;
use crate::environment::{KEY_DESTINATION, KEY_OS__PATH, KEY_OS__PRELOAD_PATH};
use crate::intercept::supervise;
use std::collections::HashMap;
use std::env::JoinPathsError;
use std::net::SocketAddr;

use std::process::ExitStatus;
use tempfile::TempDir;
use thiserror::Error;

/// Manages the environment setup for intercepting build commands during compilation.
///
/// `BuildEnvironment` is responsible for configuring the execution environment to enable
/// Bear's command interception capabilities. It supports two main interception modes:
/// - **Wrapper mode**: Creates temporary wrapper executables and modifies PATH
/// - **Preload mode**: Uses LD_PRELOAD to inject a dynamic library for system call interception
///
/// The environment includes all necessary environment variables and maintains any temporary
/// resources (like temporary directories) required for the interception to work properly.
pub struct BuildEnvironment {
    environment: HashMap<String, String>,
    _temp_dir: Option<TempDir>,
}

impl BuildEnvironment {
    /// Creates a new `BuildEnvironment` configured for the specified interception method.
    ///
    /// This method sets up the execution environment based on the provided configuration
    /// and establishes communication with the Bear process via the specified socket address.
    ///
    /// # Arguments
    ///
    /// * `config` - The interception configuration specifying the method and parameters
    /// * `address` - The socket address where the Bear process is listening for intercepted commands
    ///
    /// # Returns
    ///
    /// Returns `Ok(BuildEnvironment)` on success, or `Err(ConfigurationError)` if:
    /// - The configuration is invalid (empty paths, missing executables, etc.)
    /// - IO operations fail (creating temp directories, hard links, etc.)
    /// - Environment variable manipulation fails
    pub fn create(
        config: &config::Intercept,
        address: SocketAddr,
    ) -> Result<Self, ConfigurationError> {
        // Validate configuration first
        Self::validate_config(config)?;

        let mut environment = std::env::vars().collect::<HashMap<String, String>>();
        environment.insert(KEY_DESTINATION.to_string(), address.to_string());

        let result = match config {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                // Create temporary directory
                let temp_dir_handle = tempfile::Builder::new()
                    .prefix("bear-")
                    .tempdir_in(directory)
                    .map_err(ConfigurationError::Io)?;

                // Create hard links for all executables
                for executable in executables {
                    let link_path =
                        temp_dir_handle
                            .path()
                            .join(executable.file_name().ok_or_else(|| {
                                ConfigurationError::ConfigValidation(format!(
                                    "Invalid executable path: {}",
                                    executable.display()
                                ))
                            })?);
                    std::fs::hard_link(path, &link_path).map_err(ConfigurationError::Io)?;
                }

                // Update PATH environment variable
                let path_original = environment.get(KEY_OS__PATH).cloned().unwrap_or_default();
                let path_updated =
                    insert_to_path(&path_original, temp_dir_handle.path().to_path_buf())?;
                environment.insert(KEY_OS__PATH.to_string(), path_updated);

                Self {
                    environment,
                    _temp_dir: Some(temp_dir_handle),
                }
            }
            config::Intercept::Preload { path } => {
                // Update LD_PRELOAD environment variable
                let preload_original = environment
                    .get(KEY_OS__PRELOAD_PATH)
                    .cloned()
                    .unwrap_or_default();
                let preload_updated = insert_to_path(&preload_original, path.clone())?;
                environment.insert(KEY_OS__PRELOAD_PATH.to_string(), preload_updated);

                Self {
                    environment,
                    _temp_dir: None,
                }
            }
        };

        Ok(result)
    }

    /// Validates the provided interception configuration for correctness.
    ///
    /// This method performs comprehensive validation of the configuration parameters
    /// to ensure they are suitable for setting up the build environment. It checks
    /// for common configuration errors before attempting to create the environment.
    ///
    /// # Arguments
    ///
    /// * `config` - The interception configuration to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the configuration is valid, or `Err(ConfigurationError)`
    /// if any validation rules are violated.
    ///
    /// # Validation Rules
    ///
    /// **For Wrapper mode:**
    /// - Wrapper path must not be empty
    /// - Directory path must not be empty
    /// - At least one executable must be specified
    /// - All executable paths must not be empty
    ///
    /// **For Preload mode:**
    /// - Library path must not be empty
    fn validate_config(config: &config::Intercept) -> Result<(), ConfigurationError> {
        match config {
            config::Intercept::Wrapper {
                path,
                directory,
                executables,
            } => {
                if Self::is_empty_path(path) {
                    return Err(ConfigurationError::ConfigValidation(
                        "The wrapper path cannot be empty.".to_string(),
                    ));
                }
                if Self::is_empty_path(directory) {
                    return Err(ConfigurationError::ConfigValidation(
                        "The wrapper directory cannot be empty.".to_string(),
                    ));
                }
                if executables.is_empty() {
                    return Err(ConfigurationError::ConfigValidation(
                        "At least one executable must be specified for wrapper mode.".to_string(),
                    ));
                }
                for executable in executables {
                    if Self::is_empty_path(executable) {
                        return Err(ConfigurationError::ConfigValidation(
                            "The executable path cannot be empty.".to_string(),
                        ));
                    }
                }
                Ok(())
            }
            config::Intercept::Preload { path } => {
                if Self::is_empty_path(path) {
                    return Err(ConfigurationError::ConfigValidation(
                        "The preload library path cannot be empty.".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Checks if a given path is empty (contains only an empty string).
    ///
    /// This is a utility method used during configuration validation to detect
    /// paths that have been set to empty strings, which are considered invalid.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to check for emptiness
    ///
    /// # Returns
    ///
    /// Returns `true` if the path converts to an empty string, `false` otherwise.
    /// If the path cannot be converted to a string, returns `false`.
    fn is_empty_path(path: &std::path::Path) -> bool {
        path.to_str().is_some_and(|p| p.is_empty())
    }

    /// Converts a `BuildCommand` into a `std::process::Command` with the intercepted environment.
    ///
    /// This method creates a system command from the build command arguments and applies
    /// the configured environment variables (including interception setup) to enable
    /// command monitoring during execution.
    ///
    /// # Arguments
    ///
    /// * `val` - The build command containing the executable and arguments to run
    ///
    /// # Returns
    ///
    /// Returns a `std::process::Command` configured with the build command's arguments
    /// and the interception environment variables.
    ///
    /// # Panics
    ///
    /// Panics if the build command has no arguments (empty arguments vector).
    fn as_command(&self, val: BuildCommand) -> std::process::Command {
        let mut command = std::process::Command::new(val.arguments.first().unwrap());
        command.args(val.arguments.iter().skip(1));
        command.envs(self.environment.clone());
        command
    }

    /// Executes a build command within the configured interception environment.
    ///
    /// This is the main entry point for running build commands with Bear's interception
    /// capabilities enabled. The method sets up the command with the proper environment
    /// and delegates execution to the supervision system for monitoring and control.
    ///
    /// # Arguments
    ///
    /// * `build_command` - The build command to execute with interception enabled
    ///
    /// # Returns
    ///
    /// Returns `Ok(ExitStatus)` containing the exit status of the build command,
    /// or `Err(SuperviseError)` if the command execution fails or cannot be supervised.
    pub fn run_build(
        &self,
        build_command: BuildCommand,
    ) -> Result<ExitStatus, supervise::SuperviseError> {
        log::debug!("Running build command: {build_command:?}");
        let mut command = self.as_command(build_command);
        supervise::supervise(&mut command)
    }
}

/// Error types that can occur during build environment configuration.
///
/// This enum represents the various errors that can happen when setting up
/// the build environment for command interception. Each variant provides
/// specific context about what went wrong during the configuration process.
#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Generic IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid configuration: {0}")]
    Path(#[from] JoinPathsError),
    #[error("Configuration error: {0}")]
    ConfigValidation(String),
}

/// Manipulates a `PATH`-like environment variable by inserting a path at the beginning.
///
/// This function ensures that the specified path appears first in a colon-separated
/// list of paths (like `PATH` or `LD_PRELOAD`). If the path already exists elsewhere
/// in the list, it is removed from its current position and moved to the front.
/// This guarantees that the specified path takes precedence over other paths.
///
/// # Arguments
///
/// * `original` - The original PATH-like environment variable value
/// * `first` - The path to insert at the beginning of the path list
///
/// # Returns
///
/// Returns `Ok(String)` containing the updated path list, or `Err(ConfigurationError)`
/// if path manipulation fails due to invalid characters or platform limitations.
///
/// # Behavior
///
/// - If `original` is empty, returns just the `first` path
/// - If `first` already exists in `original`, it's moved to the front
/// - If `first` doesn't exist, it's prepended to the existing paths
/// - Uses platform-appropriate path separators and handles path encoding
fn insert_to_path(original: &str, first: std::path::PathBuf) -> Result<String, ConfigurationError> {
    if original.is_empty() {
        return Ok(first.to_string_lossy().to_string());
    }

    let mut paths: Vec<_> = std::env::split_paths(original)
        .filter(|path| path != &first)
        .collect();
    paths.insert(0, first);
    std::env::join_paths(paths)
        .map(|os_string| os_string.into_string().unwrap_or_default())
        .map_err(ConfigurationError::Path)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;

    #[test]
    fn test_insert_to_path_empty_original() {
        let original = "";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first.clone()).unwrap();
        // For empty path case, we just return the path as a string
        assert_eq!(result, first.to_string_lossy());
    }

    #[test]
    fn test_insert_to_path_prepend_new() {
        // Create platform-independent paths using std::env functions
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Create expected result using platform-specific separator
        let expected = std::env::join_paths([usr_local_bin, usr_bin, bin])
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_insert_to_path_move_existing_to_front() {
        // Create platform-independent paths using std::env functions
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_bin.clone(), usr_local_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Create expected result using platform-specific separator
        let expected = std::env::join_paths([usr_local_bin, usr_bin, bin])
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_insert_to_path_already_first() {
        // Create platform-independent paths using std::env functions
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_local_bin.clone(), usr_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        assert_eq!(result, original);
    }

    #[test]
    fn test_build_environment_validate_wrapper_valid() {
        let config = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from("/tmp"),
            executables: vec![PathBuf::from("/usr/bin/cc")],
        };
        assert!(BuildEnvironment::validate_config(&config).is_ok());
    }

    #[test]
    fn test_build_environment_validate_wrapper_empty_path() {
        let config = config::Intercept::Wrapper {
            path: PathBuf::from(""),
            directory: PathBuf::from("/tmp"),
            executables: vec![PathBuf::from("/usr/bin/cc")],
        };
        assert!(BuildEnvironment::validate_config(&config).is_err());
    }

    #[test]
    fn test_build_environment_validate_wrapper_empty_directory() {
        let config = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from(""),
            executables: vec![PathBuf::from("/usr/bin/cc")],
        };
        assert!(BuildEnvironment::validate_config(&config).is_err());
    }

    #[test]
    fn test_build_environment_validate_wrapper_empty_executables() {
        let config = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from("/tmp"),
            executables: vec![],
        };
        assert!(BuildEnvironment::validate_config(&config).is_err());
    }

    #[test]
    fn test_build_environment_validate_wrapper_empty_executable_path() {
        let config = config::Intercept::Wrapper {
            path: PathBuf::from("/usr/bin/wrapper"),
            directory: PathBuf::from("/tmp"),
            executables: vec![
                PathBuf::from("/usr/bin/cc"),
                PathBuf::from("/usr/bin/c++"),
                PathBuf::from(""),
            ],
        };
        assert!(BuildEnvironment::validate_config(&config).is_err());
    }

    #[test]
    fn test_build_environment_validate_preload_valid() {
        let config = config::Intercept::Preload {
            path: PathBuf::from("/usr/local/lib/libexec.so"),
        };
        assert!(BuildEnvironment::validate_config(&config).is_ok());
    }

    #[test]
    fn test_build_environment_validate_preload_empty_path() {
        let config = config::Intercept::Preload {
            path: PathBuf::from(""),
        };
        assert!(BuildEnvironment::validate_config(&config).is_err());
    }

    #[test]
    fn test_build_environment_create_preload() {
        let config = config::Intercept::Preload {
            path: PathBuf::from("/usr/local/lib/libintercept.so"),
        };
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let env = BuildEnvironment::create(&config, address).unwrap();

        // Check that destination is set
        assert_eq!(
            env.environment.get(KEY_DESTINATION),
            Some(&"127.0.0.1:8080".to_string())
        );

        // Check that LD_PRELOAD contains our library
        let ld_preload = env.environment.get(KEY_OS__PRELOAD_PATH).unwrap();
        assert!(ld_preload.starts_with("/usr/local/lib/libintercept.so"));
    }

    #[test]
    fn test_build_environment_create_wrapper() {
        use tempfile::TempDir;

        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a dummy wrapper executable
        let wrapper_path = temp_path.join("wrapper");
        std::fs::write(&wrapper_path, "#!/bin/bash\necho wrapper").unwrap();

        let config = config::Intercept::Wrapper {
            path: wrapper_path.clone(),
            directory: temp_path.to_path_buf(),
            executables: vec![
                std::path::PathBuf::from("gcc"),
                std::path::PathBuf::from("clang"),
            ],
        };
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        let env = BuildEnvironment::create(&config, address).unwrap();

        // Check that destination is set
        assert_eq!(
            env.environment.get(KEY_DESTINATION),
            Some(&"127.0.0.1:8080".to_string())
        );

        // Check that PATH is updated (should contain our temp directory at the beginning)
        let path = env.environment.get("PATH").unwrap();
        assert!(
            path.contains(&"bear-".to_string()),
            "PATH should contain bear temp directory: {path}"
        );

        // Verify temp directory is kept alive
        assert!(env._temp_dir.is_some());
    }
}

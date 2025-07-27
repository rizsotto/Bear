// SPDX-License-Identifier: GPL-3.0-or-later

use crate::args::BuildCommand;
use crate::config;
use crate::intercept::supervise;
use crate::intercept::{KEY_DESTINATION, KEY_PRELOAD_PATH};
use std::collections::HashMap;
use std::env::JoinPathsError;
use std::net::SocketAddr;

use std::process::ExitStatus;
use tempfile::TempDir;
use thiserror::Error;

pub struct BuildEnvironment {
    environment: HashMap<String, String>,
    _temp_dir: Option<TempDir>, // Keep tempdir alive for wrapper mode
}

impl BuildEnvironment {
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
                let path_original = environment.get("PATH").cloned().unwrap_or_default();
                let path_updated =
                    insert_to_path(&path_original, temp_dir_handle.path().to_path_buf())?;
                environment.insert("PATH".to_string(), path_updated);

                Self {
                    environment,
                    _temp_dir: Some(temp_dir_handle),
                }
            }
            config::Intercept::Preload { path } => {
                // Update LD_PRELOAD environment variable
                let preload_original = environment
                    .get(KEY_PRELOAD_PATH)
                    .cloned()
                    .unwrap_or_default();
                let preload_updated = insert_to_path(&preload_original, path.clone())?;
                environment.insert(KEY_PRELOAD_PATH.to_string(), preload_updated);

                Self {
                    environment,
                    _temp_dir: None,
                }
            }
        };

        Ok(result)
    }

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

    fn is_empty_path(path: &std::path::Path) -> bool {
        path.to_str().is_some_and(|p| p.is_empty())
    }

    fn as_command(&self, val: BuildCommand) -> std::process::Command {
        let mut command = std::process::Command::new(val.arguments.first().unwrap());
        command.args(val.arguments.iter().skip(1));
        command.envs(self.environment.clone());
        command
    }

    pub fn run_build(
        &self,
        build_command: BuildCommand,
    ) -> Result<ExitStatus, supervise::SuperviseError> {
        log::debug!("Running build command: {build_command:?}");
        let mut command = self.as_command(build_command);
        supervise::supervise(&mut command)
    }
}

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Generic IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid configuration: {0}")]
    Path(#[from] JoinPathsError),
    #[error("Configuration error: {0}")]
    ConfigValidation(String),
}

/// Manipulate a `PATH`-like environment value by inserting the `first` path into
/// the original value. It removes the `first` path if it already exists in the
/// original value. And it inserts the `first` path at the beginning of the value.
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
        let result = insert_to_path(original, first).unwrap();
        assert_eq!(result, "/usr/local/bin");
    }

    #[test]
    fn test_insert_to_path_prepend_new() {
        let original = "/usr/bin:/bin";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first).unwrap();
        assert_eq!(result, "/usr/local/bin:/usr/bin:/bin");
    }

    #[test]
    fn test_insert_to_path_move_existing_to_front() {
        let original = "/usr/bin:/usr/local/bin:/bin";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first).unwrap();
        assert_eq!(result, "/usr/local/bin:/usr/bin:/bin");
    }

    #[test]
    fn test_insert_to_path_already_first() {
        let original = "/usr/local/bin:/usr/bin:/bin";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first).unwrap();
        assert_eq!(result, "/usr/local/bin:/usr/bin:/bin");
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
        let ld_preload = env.environment.get(KEY_PRELOAD_PATH).unwrap();
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
        std::fs::set_permissions(&wrapper_path, {
            use std::os::unix::fs::PermissionsExt;
            PermissionsExt::from_mode(0o755)
        })
        .unwrap();

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

// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is responsible for formatting paths in the compiler calls.
//! The reason for this is to ensure that the paths are in a consistent format
//! when it comes to the output.
//!
//! The JSON compilation database
//! [format specification](https://clang.llvm.org/docs/JSONCompilationDatabase.html#format)
//! allows the `directory` attribute to be absolute or relative to the current working
//! directory. The `file`, `output` and `arguments` attributes are either absolute or
//! relative to the `directory` attribute.
//!
//! The `arguments` attribute contains the compiler flags, where some flags are using
//! file paths. In the current implementation, the `arguments` attribute is not
//! transformed.

use crate::config::{PathFormat, PathResolver};
use std::io;
use std::path::{absolute, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(PathBuf, PathBuf),
}

#[derive(Debug, Error)]
pub enum FormatConfigurationError {
    #[error("Invalid path format configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Getting current directory failed: {0}")]
    CurrentWorkingDirectory(#[from] io::Error),
}

/// Trait for formatting paths according to different strategies.
/// This trait allows for easy mocking in tests and provides a clean abstraction
/// for path transformation logic.
#[cfg_attr(test, mockall::automock)]
pub trait PathFormatter: Send + Sync {
    /// Format a directory path according to the configured strategy.
    fn format_directory(
        &self,
        working_dir: &Path,
        directory: &Path,
    ) -> Result<PathBuf, FormatError>;

    /// Format a file path according to the configured strategy.
    fn format_file(&self, directory: &Path, file: &Path) -> Result<PathBuf, FormatError>;
}

/// Implementation of PathFormatter that uses the configuration to determine
/// how to format paths.
pub struct ConfigurablePathFormatter {
    config: PathFormat,
}

impl ConfigurablePathFormatter {
    /// Creates a new PathFormatter with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `FormatConfigurationError::InvalidConfiguration` if the path format
    /// configuration violates the rules.
    /// Returns `FormatConfigurationError::CurrentWorkingDirectory` if getting
    /// the current working directory fails.
    pub fn new(config: PathFormat) -> Result<Self, FormatConfigurationError> {
        // Validate configuration rules
        Self::validate_path_format_config(&config)?;

        Ok(Self { config })
    }

    /// Validates the path format configuration according to the rules:
    /// - When directory is relative, file must be relative too
    /// - When directory is canonical, file can't be absolute
    /// - When directory is absolute, file can't be canonical
    fn validate_path_format_config(config: &PathFormat) -> Result<(), FormatConfigurationError> {
        use PathResolver::*;

        match (&config.directory, &config.file) {
            (Relative, Absolute | Canonical) => {
                Err(FormatConfigurationError::InvalidConfiguration(
                    "When directory is relative, file must be relative too".to_string(),
                ))
            }
            (Canonical, Absolute) => Err(FormatConfigurationError::InvalidConfiguration(
                "When directory is canonical, file can't be absolute".to_string(),
            )),
            (Absolute, Canonical) => Err(FormatConfigurationError::InvalidConfiguration(
                "When directory is absolute, file can't be canonical".to_string(),
            )),
            _ => Ok(()),
        }
    }
}

impl PathFormatter for ConfigurablePathFormatter {
    fn format_directory(
        &self,
        working_dir: &Path,
        directory: &Path,
    ) -> Result<PathBuf, FormatError> {
        self.config.directory.resolve(working_dir, directory)
    }

    fn format_file(&self, directory: &Path, file: &Path) -> Result<PathBuf, FormatError> {
        self.config.file.resolve(directory, file)
    }
}

impl PathResolver {
    /// Resolves a path according to the resolver strategy.
    ///
    /// # Parameters
    ///
    /// * `base` - The base directory for relative path calculations
    /// * `path` - The path to resolve
    ///
    /// # Returns
    ///
    /// The resolved path according to the strategy
    pub fn resolve(&self, base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
        match self {
            PathResolver::AsIs => Ok(path.to_path_buf()),
            PathResolver::Canonical => {
                let result = path.canonicalize()?;
                Ok(result)
            }
            PathResolver::Relative => {
                let absolute = absolute_to(base, path)?;
                relative_to(base, &absolute)
            }
            PathResolver::Absolute => absolute_to(base, path),
        }
    }
}

/// Compute the absolute path from the root directory if the path is relative.
fn absolute_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    if path.is_absolute() {
        Ok(absolute(path)?)
    } else {
        Ok(absolute(root.join(path))?)
    }
}

/// Compute the relative path from the root directory.
fn relative_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    // Ensure both paths are absolute for consistent behavior
    let abs_root = absolute(root)?;
    let abs_path = absolute(path)?;

    let mut root_components = abs_root.components();
    let mut path_components = abs_path.components();

    let mut remaining_root_components = Vec::new();
    let mut remaining_path_components = Vec::new();

    // Find the common prefix
    loop {
        let root_comp = root_components.next();
        let path_comp = path_components.next();
        match (root_comp, path_comp) {
            (Some(root), Some(path)) if root != path => {
                remaining_root_components.push(root);
                remaining_root_components.extend(root_components);
                remaining_path_components.push(path);
                remaining_path_components.extend(path_components);
                break;
            }
            (Some(root), None) => {
                remaining_root_components.push(root);
                remaining_root_components.extend(root_components);
                break;
            }
            (None, Some(path)) => {
                remaining_path_components.push(path);
                remaining_path_components.extend(path_components);
                break;
            }
            (None, None) => break,
            _ => continue,
        }
    }

    // Count remaining components in the root to determine how many `..` are needed
    let mut result = PathBuf::new();
    for _ in remaining_root_components {
        result.push(std::path::Component::ParentDir);
    }

    // Add the remaining components of the path
    for comp in remaining_path_components {
        // if comp is a Prefix or RootDir, signal error
        match comp {
            std::path::Component::Normal(_) | std::path::Component::ParentDir => {
                result.push(comp);
            }
            std::path::Component::CurDir => {
                // Ignore this (should not happen since we are working with absolute paths)
            }
            _ => {
                return Err(FormatError::PathsCannotBeRelative(abs_path, abs_root));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PathResolver;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_path_format_validation_success() {
        let valid_configs = vec![
            PathFormat {
                directory: PathResolver::AsIs,
                file: PathResolver::AsIs,
            },
            PathFormat {
                directory: PathResolver::Relative,
                file: PathResolver::Relative,
            },
            PathFormat {
                directory: PathResolver::Canonical,
                file: PathResolver::Relative,
            },
            PathFormat {
                directory: PathResolver::Absolute,
                file: PathResolver::Relative,
            },
            PathFormat {
                directory: PathResolver::Absolute,
                file: PathResolver::Absolute,
            },
        ];

        for config in valid_configs {
            assert!(
                ConfigurablePathFormatter::validate_path_format_config(&config).is_ok(),
                "Config should be valid: {:?}",
                config
            );
        }
    }

    #[test]
    fn test_path_format_validation_failures() {
        let invalid_configs = vec![
            (
                PathFormat {
                    directory: PathResolver::Relative,
                    file: PathResolver::Absolute,
                },
                "When directory is relative, file must be relative too",
            ),
            (
                PathFormat {
                    directory: PathResolver::Relative,
                    file: PathResolver::Canonical,
                },
                "When directory is relative, file must be relative too",
            ),
            (
                PathFormat {
                    directory: PathResolver::Canonical,
                    file: PathResolver::Absolute,
                },
                "When directory is canonical, file can't be absolute",
            ),
            (
                PathFormat {
                    directory: PathResolver::Absolute,
                    file: PathResolver::Canonical,
                },
                "When directory is absolute, file can't be canonical",
            ),
        ];

        for (config, expected_error) in invalid_configs {
            let result = ConfigurablePathFormatter::validate_path_format_config(&config);
            assert!(result.is_err(), "Config should be invalid: {:?}", config);
            if let Err(FormatConfigurationError::InvalidConfiguration(msg)) = result {
                assert_eq!(msg, expected_error);
            } else {
                panic!("Expected InvalidConfiguration error");
            }
        }
    }

    #[test]
    fn test_configurable_path_formatter_new_valid() {
        let config = PathFormat {
            directory: PathResolver::AsIs,
            file: PathResolver::AsIs,
        };

        let formatter = ConfigurablePathFormatter::new(config);
        assert!(formatter.is_ok());
    }

    #[test]
    fn test_configurable_path_formatter_new_invalid() {
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Absolute,
        };

        let formatter = ConfigurablePathFormatter::new(config);
        assert!(formatter.is_err());
    }

    #[test]
    fn test_relative_to_with_relative_paths() {
        // Test that relative_to works correctly with relative input paths
        let root = Path::new("./some/root");
        let path = Path::new("./some/path/file.txt");

        let result = relative_to(root, path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("../path/file.txt"));
    }

    #[test]
    fn test_path_resolver_as_is() {
        let resolver = PathResolver::AsIs;
        let base = PathBuf::from("/base");
        let path = PathBuf::from("some/path");

        let result = resolver.resolve(&base, &path).unwrap();
        assert_eq!(result, path);
    }

    #[test]
    fn test_absolute_to() {
        // The test creates a temporary directory and a file in it.
        // Then it verifies that the absolute path of the file is correct.
        //
        // E.g., `/tmp/tmpdir/file.txt` is the absolute path of the file,
        // if `/tmp/tmpdir` is the root directory and `file.txt` is the file.
        let root_dir = tempdir().unwrap();
        let root_dir_path = root_dir.path().canonicalize().unwrap();

        let file_path = root_dir_path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let file_relative_path = PathBuf::from("file.txt");

        let result = absolute_to(&root_dir_path, &file_relative_path).unwrap();
        assert_eq!(result, file_path);

        let result = absolute_to(&root_dir_path, &file_path).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_relative_to() {
        // The test creates two temporary directories and a file in the first one.
        // Then it verifies that the relative path from the second directory to the file
        // in the first directory is correct.
        //
        // E.g., `../tmpdir/file.txt` is the relative path to the file,
        // if `/tmp/tmpdir2` is the root directory and `/tmp/tmpdir/file.txt` is the file.
        let a_dir = tempdir().unwrap();
        let a_dir_path = a_dir.path().canonicalize().unwrap();
        let a_dir_name = a_dir_path.file_name().unwrap();

        let file_path = a_dir_path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let b_dir = tempdir().unwrap();
        let b_dir_path = b_dir.path().canonicalize().unwrap();

        let result = relative_to(&b_dir_path, &file_path).unwrap();
        assert_eq!(
            result,
            PathBuf::from("..").join(a_dir_name).join("file.txt")
        );

        let result = relative_to(&a_dir_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    #[test]
    fn test_path_formatter_format_directory() {
        let config = PathFormat {
            directory: PathResolver::AsIs,
            file: PathResolver::AsIs,
        };
        let formatter = ConfigurablePathFormatter::new(config).unwrap();

        let working_dir = PathBuf::from("/working");
        let directory = PathBuf::from("/some/dir");

        let result = formatter
            .format_directory(&working_dir, &directory)
            .unwrap();
        assert_eq!(result, directory);
    }

    #[test]
    fn test_path_formatter_format_file() {
        let config = PathFormat {
            directory: PathResolver::AsIs,
            file: PathResolver::AsIs,
        };
        let formatter = ConfigurablePathFormatter::new(config).unwrap();

        let directory = PathBuf::from("/some/dir");
        let file = PathBuf::from("file.c");

        let result = formatter.format_file(&directory, &file).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn test_path_resolver_absolute_with_temp_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let resolver = PathResolver::Absolute;
        let base = temp_path.clone();
        let relative_file = PathBuf::from("test.txt");

        let result = resolver.resolve(&base, &relative_file).unwrap();
        assert_eq!(result, file_path);
        assert!(result.is_absolute());
    }

    #[test]
    fn test_path_resolver_relative_with_temp_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let resolver = PathResolver::Relative;
        let result = resolver.resolve(&temp_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("test.txt"));
    }

    #[test]
    fn test_path_resolver_canonical_with_temp_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let resolver = PathResolver::Canonical;

        // Test with the full file path since canonicalize requires the file to exist
        let result = resolver.resolve(&temp_path, &file_path).unwrap();
        assert_eq!(result, file_path);
        assert!(result.is_absolute());
    }
}

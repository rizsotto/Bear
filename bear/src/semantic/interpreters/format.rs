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
//!
//! Formatting interpreter that wraps another interpreter to format paths in compiler commands
//! according to configuration (absolute, relative, canonical).

use crate::config::{PathFormat, PathResolver};
use crate::semantic::{
    ArgumentGroup, ArgumentKind, Command, CompilerCommand, Execution, Interpreter,
};
use std::path::{Path, PathBuf};
use std::{env, io};
use thiserror::Error;

/// A wrapper interpreter that applies path formatting to recognized compiler commands.
pub(super) struct FormattingInterpreter<T: Interpreter> {
    inner: T,
    formatter: PathFormatter,
}

impl<T: Interpreter> FormattingInterpreter<T> {
    /// Creates a formatting interpreter with the given path formatter.
    pub fn new(inner: T, formatter: PathFormatter) -> Self {
        Self { inner, formatter }
    }
}

impl<T: Interpreter> Interpreter for FormattingInterpreter<T> {
    /// This function formats the recognized command if a formatter is configured.
    ///
    /// Implemented as an Interpreter trait method, because it wraps another interpreter
    /// and applies formatting to the result semantic.
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // First, let the inner interpreter recognize the command
        let command = self.inner.recognize(execution)?;

        // Apply formatting to the command
        match command {
            Command::Compiler(compiler_cmd) => {
                // Apply formatting to the compiler command
                match self.formatter.format_command(compiler_cmd.clone()) {
                    Ok(formatted_cmd) => Some(Command::Compiler(formatted_cmd)),
                    // If formatting fails, return None
                    Err(_) => None,
                }
            }
            // Pass through other command types unchanged
            other => Some(other),
        }
    }
}

/// Represents the path formatting configuration.
pub(super) struct PathFormatter {
    config: PathFormat,
    current_dir: PathBuf,
}

impl PathFormatter {
    /// Formats the compiler command according to the configuration.
    fn format_command(&self, cmd: CompilerCommand) -> Result<CompilerCommand, FormatError> {
        // Make sure the working directory is absolute, so we can resolve paths correctly,
        // independently how the path format is requested.
        let canonic_working_dir = cmd.working_dir.canonicalize()?;

        // Format the working directory
        let working_dir = self.format_working_dir(&canonic_working_dir)?;

        // Format paths in arguments
        let arguments = cmd
            .arguments
            .iter()
            .flat_map(|argument| self.format_argument(argument, &canonic_working_dir))
            .collect();

        Ok(CompilerCommand {
            executable: cmd.executable,
            working_dir,
            arguments,
        })
    }

    fn format_working_dir(&self, working_dir: &Path) -> Result<PathBuf, FormatError> {
        self.config
            .directory
            .resolve(&self.current_dir, working_dir)
    }

    fn format_argument(
        &self,
        arg_group: &ArgumentGroup,
        working_dir: &Path,
    ) -> Result<ArgumentGroup, FormatError> {
        match arg_group.kind {
            ArgumentKind::Source => {
                if arg_group.args.len() != 1 {
                    panic!("source argument must have exactly one argument");
                }

                let source = &arg_group.args[0];
                Ok(ArgumentGroup {
                    args: vec![self.format_file(source.as_str(), working_dir)?],
                    kind: ArgumentKind::Source,
                })
            }
            ArgumentKind::Output => {
                if arg_group.args.len() != 2 {
                    panic!("output argument must have exactly two arguments");
                }

                let output = &arg_group.args[1];
                Ok(ArgumentGroup {
                    args: vec![
                        arg_group.args[0].clone(), // Keep the first argument (e.g., "-o")
                        self.format_file(output.as_str(), working_dir)?,
                    ],
                    kind: ArgumentKind::Output,
                })
            }
            _ => {
                // Don't format other argument types for now
                // In the future, we might want to format include paths, etc.
                Ok(arg_group.clone())
            }
        }
    }

    fn format_file(&self, path_str: &str, working_dir: &Path) -> Result<String, FormatError> {
        let path = PathBuf::from(path_str);
        let resolved = self.config.file.resolve(working_dir, &path)?;
        Ok(resolved.to_string_lossy().to_string())
    }
}

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(PathBuf, PathBuf),
}

/// Converts the `PathFormat` configuration into a `PathFormatter` instance.
///
/// This conversion checks the configuration and ensures that the paths are valid
/// according to the rules defined in the `PathResolver` enum. And it also captures
/// the current working directory to resolve relative paths correctly.
impl TryFrom<&PathFormat> for PathFormatter {
    type Error = FormatConfigurationError;

    fn try_from(config: &PathFormat) -> Result<Self, Self::Error> {
        use PathResolver::Relative;

        // When the directory is relative, the file and output must be relative too.
        if config.directory == Relative && config.file != Relative {
            return Err(FormatConfigurationError::OnlyRelativePaths);
        }

        Ok(Self {
            config: config.clone(),
            current_dir: env::current_dir()?,
        })
    }
}

#[derive(Debug, Error)]
pub enum FormatConfigurationError {
    #[error("Only relative paths for 'file' and 'output' when 'directory' is relative")]
    OnlyRelativePaths,
    #[error("Getting current directory failed: {0}")]
    CurrentWorkingDirectory(#[from] io::Error),
}

impl PathResolver {
    fn resolve(&self, base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
        match self {
            PathResolver::Canonical => {
                let result = path.canonicalize()?;
                Ok(result)
            }
            PathResolver::Relative => {
                let absolute = absolute_to(base, path)?;
                relative_to(base, &absolute)
            }
        }
    }
}

/// Compute the absolute path from the root directory if the path is relative.
fn absolute_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    // TODO: instead of calling `canonicalize` on the path, we should use
    //       `path::absolute` when the filesystem access is not allowed.
    if path.is_absolute() {
        Ok(path.canonicalize()?)
    } else {
        Ok(root.join(path).canonicalize()?)
    }
}

/// Compute the relative path from the root directory.
fn relative_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    // This is a naive implementation that assumes the root is
    // on the same filesystem/volume as the path.
    let mut root_components = root.components();
    let mut path_components = path.components();

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
                return Err(FormatError::PathsCannotBeRelative(
                    path.to_path_buf(),
                    root.to_path_buf(),
                ));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{ArgumentKind, CompilerCommand};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_path_formatter_try_from_valid_configs() {
        // Valid configuration: Canonical paths
        let config = PathFormat {
            directory: PathResolver::Canonical,
            file: PathResolver::Canonical,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PathFormatter { .. }));

        // Valid configuration: All relative paths
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Relative,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PathFormatter { .. }));
    }

    #[test]
    fn test_path_formatter_try_from_invalid_config() {
        // Invalid configuration: Relative directory with canonical file
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Canonical,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            FormatConfigurationError::OnlyRelativePaths
        ));
    }

    #[test]
    fn test_path_formatter_do_format() {
        let source_dir = tempdir().unwrap();
        let source_dir_path = source_dir.path().canonicalize().unwrap();
        let source_dir_name = source_dir_path.file_name().unwrap();
        let source_file_path = source_dir_path.join("main.c");
        fs::write(&source_file_path, "int main() {}").unwrap();

        let build_dir = tempdir().unwrap();
        let build_dir_path = build_dir.path().canonicalize().unwrap();
        let build_dir_name = build_dir_path.file_name().unwrap();
        let output_file_path = build_dir_path.join("main.o");
        fs::write(&output_file_path, "object").unwrap();

        let execution_dir = tempdir().unwrap();
        let execution_dir_path = execution_dir.path().canonicalize().unwrap();

        // The entry contains compiler call with absolute paths.
        let input = CompilerCommand::new(
            build_dir_path.clone(),
            "/usr/bin/gcc".into(),
            vec![
                ArgumentGroup {
                    args: vec![source_file_path.to_string_lossy().to_string()],
                    kind: ArgumentKind::Source,
                },
                ArgumentGroup {
                    args: vec!["-o".into(), output_file_path.to_string_lossy().to_string()],
                    kind: ArgumentKind::Output,
                },
                ArgumentGroup {
                    args: vec!["-O2".into()],
                    kind: ArgumentKind::Other(None),
                },
            ],
        );

        {
            let sut = PathFormatter {
                config: PathFormat {
                    directory: PathResolver::Canonical,
                    file: PathResolver::Canonical,
                },
                current_dir: execution_dir_path.to_path_buf(),
            };

            let expected = CompilerCommand::new(
                build_dir_path.clone(),
                input.executable.clone(),
                vec![
                    ArgumentGroup {
                        args: vec![source_file_path.to_string_lossy().to_string()],
                        kind: ArgumentKind::Source,
                    },
                    ArgumentGroup {
                        args: vec!["-o".into(), output_file_path.to_string_lossy().to_string()],
                        kind: ArgumentKind::Output,
                    },
                    ArgumentGroup {
                        args: vec!["-O2".into()],
                        kind: ArgumentKind::Other(None),
                    },
                ],
            );

            let result = sut.format_command(input.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
        {
            let sut = PathFormatter {
                config: PathFormat {
                    directory: PathResolver::Canonical,
                    file: PathResolver::Relative,
                },
                current_dir: execution_dir_path.to_path_buf(),
            };

            let relative_source_path = PathBuf::from("..").join(source_dir_name).join("main.c");
            let relative_output_path = PathBuf::from("main.o");

            let expected = CompilerCommand::new(
                build_dir_path.clone(),
                input.executable.clone(),
                vec![
                    ArgumentGroup {
                        args: vec![relative_source_path.to_string_lossy().to_string()],
                        kind: ArgumentKind::Source,
                    },
                    ArgumentGroup {
                        args: vec![
                            "-o".into(),
                            relative_output_path.to_string_lossy().to_string(),
                        ],
                        kind: ArgumentKind::Output,
                    },
                    ArgumentGroup {
                        args: vec!["-O2".into()],
                        kind: ArgumentKind::Other(None),
                    },
                ],
            );

            let result = sut.format_command(input.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
        {
            let sut = PathFormatter {
                config: PathFormat {
                    directory: PathResolver::Relative,
                    file: PathResolver::Relative,
                },
                current_dir: execution_dir_path.to_path_buf(),
            };

            let relative_build_dir_path = PathBuf::from("..").join(build_dir_name);
            let relative_source_path = PathBuf::from("..").join(source_dir_name).join("main.c");
            let relative_output_path = PathBuf::from("main.o");

            let expected = CompilerCommand::new(
                relative_build_dir_path,
                input.executable.clone(),
                vec![
                    ArgumentGroup {
                        args: vec![relative_source_path.to_string_lossy().to_string()],
                        kind: ArgumentKind::Source,
                    },
                    ArgumentGroup {
                        args: vec![
                            "-o".into(),
                            relative_output_path.to_string_lossy().to_string(),
                        ],
                        kind: ArgumentKind::Output,
                    },
                    ArgumentGroup {
                        args: vec!["-O2".into()],
                        kind: ArgumentKind::Other(None),
                    },
                ],
            );

            let result = sut.format_command(input.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_path_resolver() {
        let root_dir = tempdir().unwrap();
        let root_dir_path = root_dir.path().canonicalize().unwrap();

        let file_path = root_dir_path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let sut = PathResolver::Canonical;
        let result = sut.resolve(&root_dir_path, &file_path).unwrap();
        assert_eq!(result, file_path);

        let sut = PathResolver::Relative;
        let result = sut.resolve(&root_dir_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
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
}

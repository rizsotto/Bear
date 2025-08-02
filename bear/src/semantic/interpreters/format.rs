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
use crate::semantic::{ArgumentKind, Command, CompilerCommand, Execution, Interpreter};
use std::path::{Path, PathBuf};
use std::{env, io};
use thiserror::Error;

/// A wrapper interpreter that applies path formatting to recognized compiler commands.
pub(super) struct FormattingInterpreter<T: Interpreter> {
    inner: T,
    formatter: Option<PathFormatter>,
}

impl<T: Interpreter> FormattingInterpreter<T> {
    /// Creates a formatting interpreter from configuration.
    pub fn from_filter(inner: T, config: PathFormatter) -> Self {
        Self {
            inner,
            formatter: Some(config),
        }
    }

    /// Creates a pass-through formatting interpreter (no formatting applied).
    pub fn pass_through(inner: T) -> Self {
        Self {
            inner,
            formatter: None,
        }
    }
}

impl<T: Interpreter> Interpreter for FormattingInterpreter<T> {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // First, let the inner interpreter recognize the command
        let command = self.inner.recognize(execution)?;

        if let Some(formatter) = &self.formatter {
            // If a formatter is configured, format the command
            match command {
                Command::Compiler(compiler_cmd) => {
                    // Apply formatting to the compiler command
                    match formatter.format_command(compiler_cmd.clone()) {
                        Ok(formatted_cmd) => Some(Command::Compiler(formatted_cmd)),
                        // If formatting fails, return None
                        Err(_) => None,
                    }
                }
                // Pass through other command types unchanged
                other => Some(other),
            }
        } else {
            // If no formatter is configured, return the command as is
            Some(command)
        }
    }
}

pub(super) struct PathFormatter {
    config: PathFormat,
    current_dir: PathBuf,
}

impl PathFormatter {
    fn format_command(&self, mut cmd: CompilerCommand) -> Result<CompilerCommand, FormatError> {
        // Format the working directory
        let working_dir = cmd.working_dir.canonicalize()?;
        cmd.working_dir = self
            .config
            .directory
            .resolve(&self.current_dir, &working_dir)?;

        // Format paths in arguments
        for arg_group in &mut cmd.arguments {
            match arg_group.kind {
                ArgumentKind::Source => {
                    // Format source file paths
                    for arg in &mut arg_group.args {
                        if let Ok(formatted) =
                            Self::format_path(arg, &working_dir, &self.config.file)
                        {
                            *arg = formatted;
                        }
                    }
                }
                ArgumentKind::Output => {
                    // Format output file paths
                    // For output arguments, we need to handle "-o filename" pairs
                    if arg_group.args.len() >= 2 && arg_group.args[0] == "-o" {
                        if let Ok(formatted) =
                            Self::format_path(&arg_group.args[1], &working_dir, &self.config.output)
                        {
                            arg_group.args[1] = formatted;
                        }
                    }
                }
                _ => {
                    // Don't format other argument types for now
                    // In the future, we might want to format include paths, etc.
                }
            }
        }

        Ok(cmd)
    }

    fn format_path(
        path_str: &str,
        working_dir: &Path,
        resolver: &PathResolver,
    ) -> Result<String, FormatError> {
        let path = PathBuf::from(path_str);
        let resolved = resolver.resolve(working_dir, &path)?;
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

impl TryFrom<&PathFormat> for PathFormatter {
    type Error = FormatConfigurationError;

    fn try_from(config: &PathFormat) -> Result<Self, Self::Error> {
        use PathResolver::Relative;

        // When the directory is relative, the file and output must be relative too.
        if config.directory == Relative && (config.file != Relative || config.output != Relative) {
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
    use crate::semantic::ArgumentGroup;
    use crate::semantic::MockInterpreter;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_pass_through_formatting() {
        let mock_cmd = CompilerCommand::from_strings(
            "/project",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["main.c"])],
        );
        let expected_cmd = mock_cmd.clone();

        let mut mock_interpreter = MockInterpreter::new();
        mock_interpreter
            .expect_recognize()
            .returning(move |_| Some(Command::Compiler(mock_cmd.clone())));

        let sut = FormattingInterpreter::pass_through(mock_interpreter);

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "main.c"],
            "/project",
            HashMap::new(),
        );

        let result = sut.recognize(&execution);
        if let Some(Command::Compiler(result_cmd)) = result {
            assert_eq!(result_cmd, expected_cmd);
        } else {
            panic!("Expected Command::Compiler, got {result:?}");
        }
    }

    #[test]
    fn test_pass_through_non_compiler_commands() {
        let mut mock_interpreter = MockInterpreter::new();
        mock_interpreter
            .expect_recognize()
            .returning(|_| Some(Command::Ignored("test reason")));

        let sut = FormattingInterpreter::pass_through(mock_interpreter);

        let execution =
            Execution::from_strings("/usr/bin/ls", vec!["ls"], "/project", HashMap::new());

        let result = sut.recognize(&execution);
        assert!(matches!(result, Some(Command::Ignored(_))));
    }

    #[test]
    fn test_path_formatter_try_from_valid_configs() {
        // Valid configuration: Canonical paths
        let config = PathFormat {
            directory: PathResolver::Canonical,
            file: PathResolver::Canonical,
            output: PathResolver::Canonical,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PathFormatter { .. }));

        // Valid configuration: All relative paths
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Relative,
            output: PathResolver::Relative,
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
            output: PathResolver::Relative,
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
                    output: PathResolver::Canonical,
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
                    output: PathResolver::Relative,
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
                    output: PathResolver::Relative,
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

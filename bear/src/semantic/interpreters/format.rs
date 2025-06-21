// SPDX-License-Identifier: GPL-3.0-or-later

//! Formatting interpreter that wraps another interpreter to format paths in compiler commands
//! according to configuration (absolute, relative, canonical).

use crate::config::{PathFormat, PathResolver};
use crate::semantic::command::{ArgumentKind, CompilerCommand};
use crate::semantic::{Command, Execution, Interpreter};
use std::path::{Path, PathBuf};
use std::{env, io};
use thiserror::Error;

/// A wrapper interpreter that applies path formatting to recognized compiler commands.
pub struct FormattingInterpreter {
    inner: Box<dyn Interpreter>,
    formatter: PathFormatter,
}

#[derive(Debug)]
enum PathFormatter {
    /// Apply formatting according to the configuration
    Format {
        config: PathFormat,
        current_dir: PathBuf,
    },
    /// Skip formatting (pass through unchanged)
    Skip,
}

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(PathBuf, PathBuf),
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Only relative paths for 'file' and 'output' when 'directory' is relative")]
    OnlyRelativePaths,
    #[error("Getting current directory failed: {0}")]
    CurrentWorkingDirectory(#[from] io::Error),
}

impl FormattingInterpreter {
    /// Creates a new formatting interpreter that wraps another interpreter.
    pub fn new(inner: Box<dyn Interpreter>, formatter: PathFormatter) -> Self {
        Self { inner, formatter }
    }

    /// Creates a formatting interpreter from configuration.
    pub fn from_config(
        inner: Box<dyn Interpreter>,
        config: &PathFormat,
    ) -> Result<Self, ConfigurationError> {
        let formatter = PathFormatter::try_from(config)?;
        Ok(Self::new(inner, formatter))
    }

    /// Creates a pass-through formatting interpreter (no formatting applied).
    pub fn pass_through(inner: Box<dyn Interpreter>) -> Self {
        Self::new(inner, PathFormatter::Skip)
    }
}

impl Interpreter for FormattingInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // First, let the inner interpreter recognize the command
        let command = self.inner.recognize(execution)?;

        match command {
            Command::Compiler(compiler_cmd) => {
                // Apply formatting to the compiler command
                match self.formatter.format_command(compiler_cmd.clone()) {
                    Ok(formatted_cmd) => Some(Command::Compiler(formatted_cmd)),
                    Err(_) => {
                        // If formatting fails, return the original command
                        // This is a design choice - we could also return None or an error variant
                        Some(Command::Compiler(compiler_cmd))
                    }
                }
            }
            // Pass through other command types unchanged
            other => Some(other),
        }
    }
}

impl PathFormatter {
    fn format_command(&self, mut cmd: CompilerCommand) -> Result<CompilerCommand, FormatError> {
        match self {
            PathFormatter::Skip => Ok(cmd),
            PathFormatter::Format {
                config,
                current_dir,
            } => {
                // Format the working directory
                let working_dir = cmd.working_dir.canonicalize()?;
                cmd.working_dir = config.directory.resolve_path(current_dir, &working_dir)?;

                // Format paths in arguments
                for arg_group in &mut cmd.arguments {
                    match arg_group.kind {
                        ArgumentKind::Source => {
                            // Format source file paths
                            for arg in &mut arg_group.args {
                                if let Ok(formatted) =
                                    Self::format_source_path(arg, &working_dir, &config.file)
                                {
                                    *arg = formatted;
                                }
                            }
                        }
                        ArgumentKind::Output => {
                            // Format output file paths
                            // For output arguments, we need to handle "-o filename" pairs
                            if arg_group.args.len() >= 2 && arg_group.args[0] == "-o" {
                                if let Ok(formatted) = Self::format_output_path(
                                    &arg_group.args[1],
                                    &working_dir,
                                    &config.output,
                                ) {
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
        }
    }

    fn format_source_path(
        path_str: &str,
        working_dir: &Path,
        resolver: &PathResolver,
    ) -> Result<String, FormatError> {
        let path = PathBuf::from(path_str);
        let resolved = resolver.resolve_path(working_dir, &path)?;
        Ok(resolved.to_string_lossy().to_string())
    }

    fn format_output_path(
        path_str: &str,
        working_dir: &Path,
        resolver: &PathResolver,
    ) -> Result<String, FormatError> {
        let path = PathBuf::from(path_str);
        let resolved = resolver.resolve_path(working_dir, &path)?;
        Ok(resolved.to_string_lossy().to_string())
    }
}

impl TryFrom<&PathFormat> for PathFormatter {
    type Error = ConfigurationError;

    fn try_from(config: &PathFormat) -> Result<Self, Self::Error> {
        use PathResolver::Relative;

        // When the directory is relative, the file and output must be relative too.
        if config.directory == Relative && (config.file != Relative || config.output != Relative) {
            return Err(ConfigurationError::OnlyRelativePaths);
        }

        Ok(Self::Format {
            config: config.clone(),
            current_dir: env::current_dir()?,
        })
    }
}

impl PathResolver {
    fn resolve_path(&self, base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
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
    use crate::semantic::command::ArgumentGroup;
    use std::fs;
    use tempfile::tempdir;

    struct MockInterpreter {
        result: Option<Command>,
    }

    impl Interpreter for MockInterpreter {
        fn recognize(&self, _execution: &Execution) -> Option<Command> {
            self.result.clone()
        }
    }

    #[test]
    fn test_pass_through_formatting() {
        let mock_cmd = CompilerCommand::new(
            PathBuf::from("/project"),
            PathBuf::from("/usr/bin/gcc"),
            vec![ArgumentGroup {
                args: vec!["main.c".to_string()],
                kind: ArgumentKind::Source,
            }],
        );

        let mock_interpreter = MockInterpreter {
            result: Some(Command::Compiler(mock_cmd.clone())),
        };

        let formatter = FormattingInterpreter::pass_through(Box::new(mock_interpreter));

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec!["gcc".to_string(), "main.c".to_string()],
            working_dir: PathBuf::from("/project"),
            environment: std::collections::HashMap::new(),
        };

        let result = formatter.recognize(&execution);
        if let Some(Command::Compiler(result_cmd)) = result {
            assert_eq!(result_cmd, mock_cmd);
        } else {
            panic!("Expected Command::Compiler, got {:?}", result);
        }
    }

    #[test]
    fn test_pass_through_non_compiler_commands() {
        let mock_interpreter = MockInterpreter {
            result: Some(Command::Ignored("test reason")),
        };

        let formatter = FormattingInterpreter::pass_through(Box::new(mock_interpreter));

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/ls"),
            arguments: vec!["ls".to_string()],
            working_dir: PathBuf::from("/project"),
            environment: std::collections::HashMap::new(),
        };

        let result = formatter.recognize(&execution);
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
        assert!(matches!(result.unwrap(), PathFormatter::Format { .. }));

        // Valid configuration: All relative paths
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Relative,
            output: PathResolver::Relative,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PathFormatter::Format { .. }));
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
            ConfigurationError::OnlyRelativePaths
        ));
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

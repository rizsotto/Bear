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
    ArgumentKind, Arguments, BasicArguments, Command, CompilerCommand, Execution, Interpreter,
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
                match self.formatter.format_command(compiler_cmd) {
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
            .filter_map(|argument| {
                self.format_argument(argument.as_ref(), &canonic_working_dir)
                    .ok()
            })
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
        arg: &dyn Arguments,
        working_dir: &Path,
    ) -> Result<Box<dyn Arguments>, FormatError> {
        let path_updater: &dyn Fn(&Path) -> std::borrow::Cow<Path> =
            &|path: &Path| std::borrow::Cow::Borrowed(path);
        let args = arg.as_arguments(path_updater);

        match arg.kind() {
            ArgumentKind::Source => {
                if args.len() != 1 {
                    panic!("source argument must have exactly one argument");
                }

                let source = &args[0];
                Ok(Box::new(BasicArguments::new(
                    vec![self.format_file(source.as_str(), working_dir)?],
                    ArgumentKind::Source,
                )))
            }
            ArgumentKind::Output => {
                if args.len() != 2 {
                    panic!("output argument must have exactly two arguments");
                }

                let output = &args[1];
                Ok(Box::new(BasicArguments::new(
                    vec![
                        args[0].clone(), // Keep the first argument (e.g., "-o")
                        self.format_file(output.as_str(), working_dir)?,
                    ],
                    ArgumentKind::Output,
                )))
            }
            _ => {
                // Don't format other argument types for now
                // In the future, we might want to format include paths, etc.
                Ok(Box::new(BasicArguments::new(args, arg.kind())))
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
    use std::fs;
    use tempfile::tempdir;

    // TODO: Update tests to work with new Arguments trait system
    // Tests temporarily disabled during refactoring

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

// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides traits and types for recognizing the semantic meaning of executed commands
//! (such as compilers or interpreters) and for formatting their output into structured entries.

pub mod clang;
pub mod interpreters;

use super::intercept::Execution;

use std::borrow::Cow;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

/// Responsible for recognizing the semantic meaning of an executed command.
///
/// Implementers of this trait analyze an [`Execution`] and determine if it matches
/// a known command (such as a compiler or interpreter). If recognized, they
/// return a [`Command`] representing the semantic meaning of the execution.
#[cfg_attr(test, mockall::automock)]
pub trait Interpreter: Send {
    /// An [`Option<Command>`] containing the recognized command, or `None` if not recognized.
    fn recognize(&self, execution: &Execution) -> Option<Command>;
}

/// Represents a recognized command type after semantic analysis.
///
/// This enum aggregates different types of commands that can be recognized
/// by the semantic analysis system.
#[derive(Debug)]
pub enum Command {
    /// A recognized compiler command (e.g., gcc, clang).
    Compiler(CompilerCommand),
    /// A command that is intentionally ignored and not processed further.
    Ignored(&'static str),
}

/// Represents a full compiler command invocation.
///
/// The [`working_dir`] is the directory where the command is executed,
/// the [`executable`] is the path to the compiler binary,
/// while [`arguments`] contains the command-line arguments annotated
/// with their meaning (e.g., source files, output files, switches).
#[derive(Debug)]
pub struct CompilerCommand {
    pub working_dir: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<Box<dyn Arguments>>,
}

/// Trait for representing and converting compiler command arguments.
///
/// This trait provides a unified interface for handling different types of compiler arguments
/// (source files, output files, flags, etc.) and converting them to their string representations
/// for use in compilation databases or command line construction.
pub trait Arguments: std::fmt::Debug {
    /// Returns the semantic kind of this argument group.
    fn kind(&self) -> ArgumentKind;

    /// Converts this argument to its command-line string representation.
    ///
    /// # Parameters
    ///
    /// * `path_updater` - A function that can transform file paths, typically used for
    ///   making paths relative or absolute as needed for the compilation database.
    ///
    /// # Returns
    ///
    /// A vector of strings representing the command-line arguments. For example:
    /// - A source file argument might return `["main.c"]`
    /// - An output flag might return `["-o", "main.o"]`
    /// - A complex flag might return `["-std=c++17"]`
    fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String>;

    /// Extracts a file path from this argument, if applicable.
    ///
    /// This method is used to identify file arguments (typically source or output files)
    /// and extract their paths for use in compilation database entries.
    ///
    /// # Parameters
    ///
    /// * `path_updater` - A function that can transform the extracted path, typically
    ///   used for making paths relative or absolute as needed.
    ///
    /// # Returns
    ///
    /// * `Some(PathBuf)` - If this argument represents a file path
    /// * `None` - If this argument doesn't represent a file (e.g., compiler flags)
    fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf>;
}

/// Basic implementation of Arguments trait for simple argument groups
#[derive(Debug, Clone, PartialEq)]
pub struct BasicArguments {
    pub args: Vec<String>,
    pub kind: ArgumentKind,
}

impl BasicArguments {
    pub fn new(args: Vec<String>, kind: ArgumentKind) -> Self {
        Self { args, kind }
    }
}

impl Arguments for BasicArguments {
    fn kind(&self) -> ArgumentKind {
        self.kind.clone()
    }

    fn as_arguments(&self, _path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        self.args.clone()
    }

    fn as_file(&self, _path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        match self.kind {
            ArgumentKind::Source => self.args.first().map(PathBuf::from),
            ArgumentKind::Output => self.args.get(1).map(PathBuf::from),
            _ => None,
        }
    }
}

/// Specialized implementation for GCC-style source file arguments
/// Demonstrates how different compilers can have their own argument handling
#[derive(Debug, Clone, PartialEq)]
pub struct GccSourceArgument {
    pub file_path: String,
}

impl Arguments for GccSourceArgument {
    fn kind(&self) -> ArgumentKind {
        ArgumentKind::Source
    }

    fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        let path = Path::new(&self.file_path);
        let updated_path = path_updater(path);
        vec![updated_path.to_string_lossy().to_string()]
    }

    fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        let path = Path::new(&self.file_path);
        let updated_path = path_updater(path);
        Some(updated_path.into_owned())
    }
}

/// Specialized implementation for MSVC-style output arguments (/out:file.obj)
/// Demonstrates different compiler syntax handling
#[derive(Debug, Clone, PartialEq)]
pub struct MsvcOutputArgument {
    pub file_path: String,
}

impl Arguments for MsvcOutputArgument {
    fn kind(&self) -> ArgumentKind {
        ArgumentKind::Output
    }

    fn as_arguments(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        let path = Path::new(&self.file_path);
        let updated_path = path_updater(path);
        vec![format!("/out:{}", updated_path.to_string_lossy())]
    }

    fn as_file(&self, path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        let path = Path::new(&self.file_path);
        let updated_path = path_updater(path);
        Some(updated_path.into_owned())
    }
}

/// Represents the meaning of the argument in the compiler call. Identifies
/// the purpose of each argument in the command line.
///
/// Variants:
/// - `Compiler`: The compiler executable itself.
/// - `Source`: A source file to be compiled.
/// - `Output`: An output file or related argument (e.g., `-o output.o`).
/// - `Other`: Any other argument not classified above (e.g., compiler switches like `-Wall`).
///   Can optionally specify which compiler pass the argument affects.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgumentKind {
    Compiler,
    Source,
    Output,
    Other(Option<CompilerPass>),
}

/// Represents different compiler passes that an argument might affect.
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerPass {
    Info,
    Preprocessing,
    Compiling,
    Assembling,
    Linking,
}

impl CompilerCommand {
    pub fn new(
        working_dir: PathBuf,
        executable: PathBuf,
        arguments: Vec<Box<dyn Arguments>>,
    ) -> Self {
        Self {
            working_dir,
            executable,
            arguments,
        }
    }

    #[cfg(test)]
    pub fn from_strings(
        working_dir: &str,
        executable: &str,
        arguments: Vec<(ArgumentKind, Vec<&str>)>,
    ) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            executable: PathBuf::from(executable),
            arguments: arguments
                .into_iter()
                .map(|(kind, args)| {
                    Box::new(BasicArguments::new(
                        args.into_iter().map(String::from).collect(),
                        kind,
                    )) as Box<dyn Arguments>
                })
                .collect(),
        }
    }

    /// Compare two CompilerCommands by their arguments.
    /// This can be useful for deduplication or testing purposes.
    pub fn has_same_arguments(&self, other: &CompilerCommand) -> bool {
        // Compare each argument using equals method
        // Compare arguments by their string representation and kind
        if self.arguments.len() != other.arguments.len() {
            return false;
        }

        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);

        self.arguments
            .iter()
            .zip(other.arguments.iter())
            .all(|(a, b)| {
                a.kind() == b.kind()
                    && a.as_arguments(&path_updater) == b.as_arguments(&path_updater)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arguments_equality() {
        let arg1 = BasicArguments::new(vec!["main.c".to_string()], ArgumentKind::Source);
        let arg2 = BasicArguments::new(vec!["main.c".to_string()], ArgumentKind::Source);
        let arg3 = BasicArguments::new(vec!["other.c".to_string()], ArgumentKind::Source);
        let arg4 = BasicArguments::new(
            vec!["-o".to_string(), "main.o".to_string()],
            ArgumentKind::Output,
        );

        // Same arguments should be equal (using direct comparison)
        assert_eq!(arg1, arg2);

        // Different arguments should not be equal
        assert_ne!(arg1, arg3);
        assert_ne!(arg1, arg4);
    }

    #[test]
    fn test_compiler_command_comparison() {
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd3 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["other.c"]),
                (ArgumentKind::Output, vec!["-o", "other.o"]),
            ],
        );

        // Same arguments should be equal
        assert!(cmd1.has_same_arguments(&cmd2));

        // Different arguments should not be equal
        assert!(!cmd1.has_same_arguments(&cmd3));
    }

    #[test]
    fn test_specialized_arguments_equality() {
        let gcc_arg1 = GccSourceArgument {
            file_path: "main.c".to_string(),
        };
        let gcc_arg2 = GccSourceArgument {
            file_path: "main.c".to_string(),
        };
        let gcc_arg3 = GccSourceArgument {
            file_path: "other.c".to_string(),
        };

        assert_eq!(gcc_arg1, gcc_arg2);
        assert_ne!(gcc_arg1, gcc_arg3);
    }
}

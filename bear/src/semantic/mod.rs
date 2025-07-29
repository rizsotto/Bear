// SPDX-License-Identifier: GPL-3.0-or-later

//! Semantic analysis module for command execution recognition and formatting.
//!
//! This module provides traits and types for recognizing the semantic meaning of executed commands
//! (such as compilers or interpreters) and for formatting their output into structured entries.

pub mod clang;
pub mod interpreters;

use super::intercept::Execution;
use crate::config;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// A recognized compiler command (e.g., gcc, clang).
    Compiler(CompilerCommand),
    /// A command that is intentionally ignored and not processed further.
    Ignored(&'static str),
    /// A command that is filtered out and not included in the output.
    Filtered(String),
}

impl Command {
    /// Converts the command into compilation database entries.
    pub fn to_entries(&self, config: &config::EntryFormat) -> Vec<clang::Entry> {
        match self {
            Command::Compiler(cmd) => cmd.to_entries(config),
            Command::Ignored(_) => vec![],
            Command::Filtered(_) => vec![],
        }
    }
}

/// Represents a full compiler command invocation.
///
/// The [`working_dir`] is the directory where the command is executed,
/// the [`executable`] is the path to the compiler binary,
/// while [`arguments`] contains the command-line arguments annotated
/// with their meaning (e.g., source files, output files, switches).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompilerCommand {
    pub working_dir: PathBuf,
    pub executable: PathBuf,
    pub arguments: Vec<ArgumentGroup>,
}

/// Represents a group of arguments of the same intent.
///
/// Groups the arguments which belongs together, and annotate the meaning of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgumentGroup {
    pub args: Vec<String>,
    pub kind: ArgumentKind,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArgumentKind {
    Compiler,
    Source,
    Output,
    Other(Option<CompilerPass>),
}

/// Represents different compiler passes that an argument might affect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompilerPass {
    Info,
    Preprocessing,
    Compiling,
    Assembling,
    Linking,
}

impl CompilerCommand {
    pub fn new(working_dir: PathBuf, executable: PathBuf, arguments: Vec<ArgumentGroup>) -> Self {
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
                .map(|(kind, args)| ArgumentGroup {
                    args: args.into_iter().map(String::from).collect(),
                    kind,
                })
                .collect(),
        }
    }

    /// Converts the compiler command into a list of entries for the compilation database.
    ///
    /// It processes the command arguments, identifies source files, and constructs
    /// entries with the executable, arguments, working directory, and output file if present.
    pub(super) fn to_entries(&self, config: &config::EntryFormat) -> Vec<clang::Entry> {
        // Find all source files in the arguments
        let source_files: Vec<String> = self
            .arguments
            .iter()
            .filter(|arg| matches!(arg.kind, ArgumentKind::Source))
            .flat_map(|arg| &arg.args)
            .cloned()
            .collect();

        // If no source files found, return empty vector
        if source_files.is_empty() {
            return vec![];
        }

        // Build the full command arguments by flattening all argument args
        let mut command_args = vec![self.executable.to_string_lossy().to_string()];
        for arg in &self.arguments {
            command_args.extend(arg.args.iter().cloned());
        }

        // Find output file if present
        let output_file = if config.keep_output_field {
            self.arguments
                .iter()
                .filter(|arg| matches!(arg.kind, ArgumentKind::Output))
                .flat_map(|arg| &arg.args)
                .nth(1) // Skip the "-o" flag itself, take the output filename
                .map(PathBuf::from)
        } else {
            None
        };

        // Create one entry per source file
        source_files
            .into_iter()
            .map(|source_file| {
                clang::Entry::new(
                    source_file,
                    command_args.clone(),
                    &self.working_dir,
                    output_file.as_ref(),
                    !config.command_field_as_array,
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::EntryFormat;

    #[test]
    fn test_compiler_command_to_entries_single_source() {
        let sut = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (
                    ArgumentKind::Other(Some(CompilerPass::Compiling)),
                    vec!["-c"],
                ),
                (ArgumentKind::Other(None), vec!["-Wall"]),
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let config = EntryFormat::default();
        let entries = sut.to_entries(&config);

        let expected = vec![clang::Entry::from_arguments_str(
            "main.c",
            vec!["/usr/bin/gcc", "-c", "-Wall", "main.c", "-o", "main.o"],
            "/home/user",
            Some("main.o"),
        )];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_compiler_command_to_entries_multiple_sources() {
        let sut = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/g++",
            vec![
                (
                    ArgumentKind::Other(Some(CompilerPass::Compiling)),
                    vec!["-c"],
                ),
                (ArgumentKind::Source, vec!["file1.cpp"]),
                (ArgumentKind::Source, vec!["file2.cpp"]),
            ],
        );

        let config = EntryFormat::default();
        let entries = sut.to_entries(&config);

        let expected = vec![
            clang::Entry::from_arguments_str(
                "file1.cpp",
                vec!["/usr/bin/g++", "-c", "file1.cpp", "file2.cpp"],
                "/home/user",
                None,
            ),
            clang::Entry::from_arguments_str(
                "file2.cpp",
                vec!["/usr/bin/g++", "-c", "file1.cpp", "file2.cpp"],
                "/home/user",
                None,
            ),
        ];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_compiler_command_to_entries_no_sources() {
        let sut = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![(
                ArgumentKind::Other(Some(CompilerPass::Info)),
                vec!["--version"],
            )],
        );

        let config = EntryFormat::default();
        let entries = sut.to_entries(&config);

        let expected: Vec<clang::Entry> = vec![];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_to_entries_command_field_as_string() {
        let sut = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (
                    ArgumentKind::Other(Some(CompilerPass::Compiling)),
                    vec!["-c"],
                ),
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );
        let config = EntryFormat {
            keep_output_field: true,
            command_field_as_array: false,
        };
        let entries = sut.to_entries(&config);

        let expected = vec![clang::Entry::from_command_str(
            "main.c",
            "/usr/bin/gcc -c main.c -o main.o",
            "/home/user",
            Some("main.o"),
        )];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_to_entries_without_output_field() {
        let sut = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (
                    ArgumentKind::Other(Some(CompilerPass::Compiling)),
                    vec!["-c"],
                ),
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );
        let config = EntryFormat {
            command_field_as_array: true,
            keep_output_field: false,
        };
        let entries = sut.to_entries(&config);

        let expected = vec![clang::Entry::from_arguments_str(
            "main.c",
            vec!["/usr/bin/gcc", "-c", "main.c", "-o", "main.o"],
            "/home/user",
            None,
        )];
        assert_eq!(entries, expected);
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides types for representing compiler commands.
//!
//! It defines how to classify arguments, group them, and convert them into entries
//! for the final compilation database.

use crate::semantic::{clang, FormatConfig, Formattable};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Formattable for CompilerCommand {
    /// Converts the compiler command into a list of entries for the compilation database.
    ///
    /// It processes the command arguments, identifies source files, and constructs
    /// entries with the executable, arguments, working directory, and output file if present.
    fn to_entries(&self, config: &FormatConfig) -> Vec<clang::Entry> {
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
                if config.command_field_as_array {
                    clang::Entry::from_arguments(
                        source_file,
                        command_args.clone(),
                        &self.working_dir,
                        output_file.as_ref(),
                    )
                } else {
                    clang::Entry::from_arguments_as_command(
                        source_file,
                        command_args.clone(),
                        &self.working_dir,
                        output_file.as_ref(),
                    )
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

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

        let config = FormatConfig::default();
        let entries = sut.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.file, PathBuf::from("main.c"));
        assert_eq!(entry.directory, PathBuf::from("/home/user"));
        assert_eq!(
            entry.arguments,
            vec!["/usr/bin/gcc", "-c", "-Wall", "main.c", "-o", "main.o"]
        );
        assert_eq!(entry.output, Some(PathBuf::from("main.o")));
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

        let config = FormatConfig::default();
        let entries = sut.to_entries(&config);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file, PathBuf::from("file1.cpp"));
        assert_eq!(entries[1].file, PathBuf::from("file2.cpp"));

        for entry in &entries {
            assert_eq!(entry.directory, PathBuf::from("/home/user"));
            assert_eq!(
                entry.arguments,
                vec!["/usr/bin/g++", "-c", "file1.cpp", "file2.cpp"]
            );
        }
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

        let config = FormatConfig::default();
        let entries = sut.to_entries(&config);

        assert_eq!(entries.len(), 0);
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
        let config = FormatConfig {
            keep_output_field: true,
            command_field_as_array: false,
        };
        let entries = sut.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.directory, PathBuf::from("/home/user"));
        assert_eq!(entry.file, PathBuf::from("main.c"));
        assert_eq!(entry.output, Some(PathBuf::from("main.o")));
        // Command should be a string, not an array
        assert!(entry.arguments.is_empty());
        assert_eq!(entry.command, "/usr/bin/gcc -c main.c -o main.o");
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
        let config = FormatConfig {
            command_field_as_array: true,
            keep_output_field: false,
        };
        let entries = sut.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        // Output field should be None
        assert!(entry.output.is_none());
    }
}

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
/// - `Switch`: A compiler switch or flag (e.g., `-Wall`).
/// - `Other`: Any other argument not classified above.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArgumentKind {
    Compiler,
    Source,
    Output,
    Switch,
    Other,
}

impl CompilerCommand {
    pub fn new(working_dir: PathBuf, executable: PathBuf, arguments: Vec<ArgumentGroup>) -> Self {
        Self {
            working_dir,
            executable,
            arguments,
        }
    }
}

impl Formattable for CompilerCommand {
    /// Converts the compiler command into a list of entries for the compilation database.
    ///
    /// It processes the command arguments, identifies source files, and constructs
    /// entries with the executable, arguments, working directory, and output file if present.
    fn to_entries(&self, _config: &FormatConfig) -> Vec<clang::Entry> {
        // Find all source files in the arguments
        let source_files: Vec<String> = self
            .arguments
            .iter()
            .filter(|arg| arg.kind == ArgumentKind::Source)
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
        let output_file = self
            .arguments
            .iter()
            .filter(|arg| arg.kind == ArgumentKind::Output)
            .flat_map(|arg| &arg.args)
            .skip(1) // Skip the "-o" flag itself, take the output filename
            .next()
            .map(|s| PathBuf::from(s));

        // Create one entry per source file
        source_files
            .into_iter()
            .map(|source_file| {
                clang::Entry::from_arguments(
                    source_file,
                    command_args.clone(),
                    &self.working_dir,
                    output_file.as_ref(),
                )
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
        let cmd = CompilerCommand::new(
            PathBuf::from("/home/user"),
            PathBuf::from("/usr/bin/gcc"),
            vec![
                ArgumentGroup {
                    args: vec!["-c".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["-Wall".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["main.c".to_string()],
                    kind: ArgumentKind::Source,
                },
            ],
        );

        let config = FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.file, PathBuf::from("main.c"));
        assert_eq!(entry.directory, PathBuf::from("/home/user"));
        assert_eq!(
            entry.arguments,
            vec!["/usr/bin/gcc", "-c", "-Wall", "main.c"]
        );
        assert_eq!(entry.output, None);
    }

    #[test]
    fn test_compiler_command_to_entries_multiple_sources() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/home/user"),
            PathBuf::from("/usr/bin/g++"),
            vec![
                ArgumentGroup {
                    args: vec!["-c".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["file1.cpp".to_string()],
                    kind: ArgumentKind::Source,
                },
                ArgumentGroup {
                    args: vec!["file2.cpp".to_string()],
                    kind: ArgumentKind::Source,
                },
            ],
        );

        let config = FormatConfig::default();
        let entries = cmd.to_entries(&config);

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
    fn test_compiler_command_to_entries_with_output() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/tmp"),
            PathBuf::from("clang"),
            vec![
                ArgumentGroup {
                    args: vec!["-c".to_string()],
                    kind: ArgumentKind::Switch,
                },
                ArgumentGroup {
                    args: vec!["-o".to_string(), "main.o".to_string()],
                    kind: ArgumentKind::Output,
                },
                ArgumentGroup {
                    args: vec!["main.c".to_string()],
                    kind: ArgumentKind::Source,
                },
            ],
        );

        let config = FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.file, PathBuf::from("main.c"));
        assert_eq!(entry.directory, PathBuf::from("/tmp"));
        assert_eq!(
            entry.arguments,
            vec!["clang", "-c", "-o", "main.o", "main.c"]
        );
        assert_eq!(entry.output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn test_compiler_command_to_entries_no_sources() {
        let cmd = CompilerCommand::new(
            PathBuf::from("/home/user"),
            PathBuf::from("gcc"),
            vec![ArgumentGroup {
                args: vec!["--version".to_string()],
                kind: ArgumentKind::Switch,
            }],
        );

        let config = FormatConfig::default();
        let entries = cmd.to_entries(&config);

        assert_eq!(entries.len(), 0);
    }
}

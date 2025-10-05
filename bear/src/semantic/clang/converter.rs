// SPDX-License-Identifier: GPL-3.0-or-later

//! Command to compilation database entry conversion functionality.
//!
//! This module provides the [`CommandConverter`] which is responsible for converting
//! semantic [`Command`] instances into clang compilation database [`Entry`] objects.
//! The converter encapsulates format configuration and conversion logic, providing
//! a clean separation between domain objects and output formatting.
//!
//! The conversion process handles:
//! - Extracting source files from compiler command arguments
//! - Building properly formatted command lines for each source file
//! - Computing output files based on command arguments
//! - Applying format configuration (array vs string commands, output field inclusion)
//!
//! # Example
//!
//! ```rust
//! use bear::semantic::clang::converter::CommandConverter;
//! use bear::config::EntryFormat;
//!
//! let config = EntryFormat::default();
//! let converter = CommandConverter::new(config);
//!
//! // The converter can be used to convert semantic Command instances
//! // into compilation database entries based on the configured format
//! ```

use super::Entry;
use crate::config;
use crate::semantic::{ArgumentKind, Arguments, Command, CompilerCommand};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Converts commands into compilation database entries.
///
/// This converter takes format configuration during construction and uses it
/// to convert commands into appropriately formatted entries.
pub struct CommandConverter {
    format: config::EntryFormat,
}

impl CommandConverter {
    /// Creates a new CommandConverter with the specified format configuration.
    pub fn new(format: config::EntryFormat) -> Self {
        Self { format }
    }

    /// Converts the command into compilation database entries.
    pub fn to_entries(&self, command: &Command) -> Vec<Entry> {
        match command {
            Command::Compiler(cmd) => self.convert_compiler_command(cmd),
            Command::Ignored(_) => vec![],
        }
    }

    /// Converts a compiler command into compilation database entries.
    fn convert_compiler_command(&self, cmd: &CompilerCommand) -> Vec<Entry> {
        // Find all source arguments
        let source_arguments = Self::find_arguments_by_kind(cmd, ArgumentKind::Source)
            .collect::<Vec<&Box<dyn Arguments>>>();

        // If no source files found, return empty vector
        if source_arguments.is_empty() {
            return vec![];
        }

        // Find output file if present
        let output_file = if self.format.keep_output_field {
            Self::compute_output_file(cmd)
        } else {
            None
        };

        // Create one entry per source argument
        source_arguments
            .into_iter()
            .filter_map(|source_arg| {
                let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
                let source_file = source_arg.as_file(path_updater)?;

                let command_args = Self::build_command_args_for_source(cmd, source_arg.as_ref());

                if self.format.command_field_as_array {
                    Some(Entry::with_arguments(
                        source_file,
                        command_args,
                        &cmd.working_dir,
                        output_file.as_ref(),
                    ))
                } else {
                    Some(Entry::with_command(
                        source_file,
                        command_args,
                        &cmd.working_dir,
                        output_file.as_ref(),
                    ))
                }
            })
            .collect()
    }

    /// Builds command arguments for a specific source file.
    ///
    /// This method constructs the command arguments list that includes the executable,
    /// all non-source arguments, and the specific source file.
    /// It ensures that the source file is placed in the correct position relative to output arguments.
    fn build_command_args_for_source(
        cmd: &CompilerCommand,
        source_arg: &dyn Arguments,
    ) -> Vec<String> {
        // Start with the executable
        let mut command_args = vec![cmd.executable.to_string_lossy().to_string()];

        // Add all non-source arguments, while handling source file placement
        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        for arg in &cmd.arguments {
            // Skip this specific source argument (using pointer equality)
            if matches!(arg.kind(), ArgumentKind::Source) && !std::ptr::eq(arg.as_ref(), source_arg)
            {
                continue;
            }

            command_args.extend(arg.as_arguments(&path_updater));
        }

        command_args
    }

    /// Returns arguments of a specific kind from the command.
    ///
    /// This method filters arguments by their kind and returns their values as strings.
    fn find_arguments_by_kind(
        cmd: &CompilerCommand,
        kind: ArgumentKind,
    ) -> impl Iterator<Item = &Box<dyn Arguments>> {
        cmd.arguments.iter().filter(move |arg| arg.kind() == kind)
    }

    /// Computes the output file path from the command arguments.
    ///
    /// This method examines the output arguments (typically "-o filename")
    /// and returns the filename as a PathBuf.
    fn compute_output_file(cmd: &CompilerCommand) -> Option<PathBuf> {
        // Find output arguments and convert to a file path
        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        Self::find_arguments_by_kind(cmd, ArgumentKind::Output)
            .nth(0)
            .and_then(|arg| arg.as_file(path_updater))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EntryFormat;
    use crate::semantic::{ArgumentKind, Command, CompilerCommand, CompilerPass};

    #[test]
    fn test_compiler_command_to_entries_single_source() {
        let command = Command::Compiler(CompilerCommand::from_strings(
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
        ));

        let converter = CommandConverter::new(EntryFormat::default());
        let entries = converter.to_entries(&command);

        let expected = vec![Entry::from_arguments_str(
            "main.c",
            vec!["/usr/bin/gcc", "-c", "-Wall", "main.c", "-o", "main.o"],
            "/home/user",
            Some("main.o"),
        )];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_compiler_command_to_entries_multiple_sources() {
        let command = Command::Compiler(CompilerCommand::from_strings(
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
        ));

        let converter = CommandConverter::new(EntryFormat::default());
        let result = converter.to_entries(&command);

        let expected = vec![
            Entry::from_arguments_str(
                "file1.cpp",
                vec!["/usr/bin/g++", "-c", "file1.cpp"],
                "/home/user",
                None,
            ),
            Entry::from_arguments_str(
                "file2.cpp",
                vec!["/usr/bin/g++", "-c", "file2.cpp"],
                "/home/user",
                None,
            ),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compiler_command_to_entries_no_sources() {
        let command = Command::Compiler(CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![(
                ArgumentKind::Other(Some(CompilerPass::Info)),
                vec!["--version"],
            )],
        ));

        let converter = CommandConverter::new(EntryFormat::default());
        let result = converter.to_entries(&command);

        let expected: Vec<Entry> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_to_entries_command_field_as_string() {
        let command = Command::Compiler(CompilerCommand::from_strings(
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
        ));
        let config = EntryFormat {
            keep_output_field: true,
            command_field_as_array: false,
        };
        let converter = CommandConverter::new(config);
        let entries = converter.to_entries(&command);

        let expected = vec![Entry::from_command_str(
            "main.c",
            "/usr/bin/gcc -c main.c -o main.o",
            "/home/user",
            Some("main.o"),
        )];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_to_entries_without_output_field() {
        let command = Command::Compiler(CompilerCommand::from_strings(
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
        ));
        let config = EntryFormat {
            command_field_as_array: true,
            keep_output_field: false,
        };
        let sut = CommandConverter::new(config);
        let result = sut.to_entries(&command);

        let expected = vec![Entry::from_arguments_str(
            "main.c",
            vec!["/usr/bin/gcc", "-c", "main.c", "-o", "main.o"],
            "/home/user",
            None,
        )];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_command_converter_public_api() {
        // Test that CommandConverter can be used as a public API
        let config = EntryFormat {
            command_field_as_array: true,
            keep_output_field: false,
        };
        let converter = CommandConverter::new(config);

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (
                    ArgumentKind::Other(Some(CompilerPass::Compiling)),
                    vec!["-c"],
                ),
                (ArgumentKind::Source, vec!["test.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let entries = converter.to_entries(&command);

        assert_eq!(entries.len(), 1);
        // Verify the entry is valid using the public API
        assert!(entries[0].validate().is_ok());
    }
}

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
//! use bear::output::clang::converter::CommandConverter;
//! use bear::config::Format;
//!
//! let config = Format::default();
//! let converter = CommandConverter::new(config).unwrap();
//!
//! // The converter can be used to convert semantic Command instances
//! // into compilation database entries based on the configured format
//! ```

use super::Entry;
use super::{ConfigurablePathFormatter, FormatConfigurationError, PathFormatter};
use crate::config;
use crate::semantic::{ArgumentKind, Arguments, Command, CompilerCommand};
use log::warn;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Converts commands into compilation database entries.
///
/// This converter takes format configuration during construction and uses it
/// to convert commands into appropriately formatted entries.
pub struct CommandConverter {
    format: config::EntryFormat,
    path_formatter: Box<dyn PathFormatter>,
}

impl CommandConverter {
    /// Creates a new CommandConverter with the specified format configuration.
    pub fn new(format: config::Format) -> Result<Self, FormatConfigurationError> {
        let path_formatter = Box::new(ConfigurablePathFormatter::new(format.paths)?);
        Ok(Self {
            format: format.entry,
            path_formatter,
        })
    }

    /// Creates a new CommandConverter with a custom path formatter for testing.
    #[cfg(test)]
    pub fn with_formatter(
        format: config::EntryFormat,
        path_formatter: Box<dyn PathFormatter>,
    ) -> Self {
        Self {
            format,
            path_formatter,
        }
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

        // Format directory path
        let formatted_directory = match self
            .path_formatter
            .format_directory(&cmd.working_dir, &cmd.working_dir)
        {
            Ok(dir) => dir,
            Err(e) => {
                warn!("Failed to format directory path: {}", e);
                return vec![];
            }
        };

        // Find output file if present
        let output_file = if self.format.keep_output_field {
            Self::compute_output_file(cmd, &formatted_directory, &*self.path_formatter)
        } else {
            None
        };

        // Create one entry per source argument
        source_arguments
            .into_iter()
            .filter_map(|source_arg| {
                // Get source file with original path first, then format it
                let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
                let source_file_path = source_arg.as_file(path_updater)?;
                let formatted_source_file =
                    self.format_file_path(&formatted_directory, &source_file_path);

                let command_args = self.build_command_args_for_source(
                    cmd,
                    source_arg.as_ref(),
                    &formatted_directory,
                );

                if self.format.command_field_as_array {
                    Some(Entry::with_arguments(
                        formatted_source_file,
                        command_args,
                        &formatted_directory,
                        output_file.as_ref(),
                    ))
                } else {
                    Some(Entry::with_command(
                        formatted_source_file,
                        command_args,
                        &formatted_directory,
                        output_file.as_ref(),
                    ))
                }
            })
            .collect()
    }

    /// Helper method to format a file path
    fn format_file_path(&self, formatted_directory: &Path, file_path: &Path) -> PathBuf {
        match self
            .path_formatter
            .format_file(formatted_directory, file_path)
        {
            Ok(formatted_path) => formatted_path,
            Err(e) => {
                warn!("Failed to format file path {}: {}", file_path.display(), e);
                file_path.to_path_buf()
            }
        }
    }

    /// Builds command arguments for a specific source file.
    ///
    /// This method constructs the command arguments list that includes the executable,
    /// all non-source arguments, and the specific source file.
    /// It ensures that the source file is placed in the correct position relative to output arguments.
    fn build_command_args_for_source(
        &self,
        cmd: &CompilerCommand,
        source_arg: &dyn Arguments,
        formatted_directory: &Path,
    ) -> Vec<String> {
        // Start with the executable
        let mut command_args = vec![cmd.executable.to_string_lossy().to_string()];

        // Add all non-source arguments, while handling source file placement
        for arg in &cmd.arguments {
            // Skip this specific source argument (using pointer equality)
            if matches!(arg.kind(), ArgumentKind::Source) && !std::ptr::eq(arg.as_ref(), source_arg)
            {
                continue;
            }

            // Get arguments with original paths, then format any file paths
            let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
            let original_args = arg.as_arguments(path_updater);

            // For file-type arguments, we need to format the paths
            match arg.kind() {
                ArgumentKind::Source | ArgumentKind::Output => {
                    // These might contain file paths that need formatting
                    let formatted_args = original_args
                        .into_iter()
                        .map(|arg_str| {
                            let path = Path::new(&arg_str);
                            if path.is_absolute() || path.extension().is_some() {
                                // Likely a file path, format it
                                self.format_file_path(formatted_directory, path)
                                    .to_string_lossy()
                                    .to_string()
                            } else {
                                // Likely a flag, keep as-is
                                arg_str
                            }
                        })
                        .collect::<Vec<_>>();
                    command_args.extend(formatted_args);
                }
                _ => {
                    // Non-file arguments, use as-is
                    command_args.extend(original_args);
                }
            }
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
    fn compute_output_file(
        cmd: &CompilerCommand,
        formatted_directory: &Path,
        path_formatter: &dyn PathFormatter,
    ) -> Option<PathBuf> {
        // Find output arguments and get the original path first
        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        let output_path = Self::find_arguments_by_kind(cmd, ArgumentKind::Output)
            .nth(0)
            .and_then(|arg| arg.as_file(path_updater))?;

        // Format the output path
        match path_formatter.format_file(formatted_directory, &output_path) {
            Ok(formatted_path) => Some(formatted_path),
            Err(e) => {
                warn!(
                    "Failed to format output file path {}: {}",
                    output_path.display(),
                    e
                );
                Some(output_path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::clang::format::{FormatError, MockPathFormatter};
    use super::*;
    use crate::config::{EntryFormat, Format, PathFormat};
    use crate::semantic::{ArgumentKind, Command, CompilerCommand, CompilerPass};
    use std::io;

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

        let format = Format {
            paths: PathFormat::default(),
            entry: EntryFormat::default(),
        };
        let converter = CommandConverter::new(format).unwrap();
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

        let format = Format {
            paths: PathFormat::default(),
            entry: EntryFormat::default(),
        };
        let converter = CommandConverter::new(format).unwrap();
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

        let format = Format {
            paths: PathFormat::default(),
            entry: EntryFormat::default(),
        };
        let converter = CommandConverter::new(format).unwrap();
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
        let format = Format {
            paths: PathFormat::default(),
            entry: EntryFormat {
                keep_output_field: true,
                command_field_as_array: false,
            },
        };
        let converter = CommandConverter::new(format).unwrap();
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
        let format = Format {
            paths: PathFormat::default(),
            entry: EntryFormat {
                command_field_as_array: true,
                keep_output_field: false,
            },
        };
        let sut = CommandConverter::new(format).unwrap();
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
        let format = Format {
            paths: PathFormat::default(),
            entry: EntryFormat {
                command_field_as_array: true,
                keep_output_field: false,
            },
        };
        let converter = CommandConverter::new(format).unwrap();

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

    #[test]
    fn test_path_formatting_with_custom_formatter() {
        let mut mock_formatter = MockPathFormatter::new();

        // Set up expectations for the mock
        mock_formatter
            .expect_format_directory()
            .returning(|_, dir| Ok(PathBuf::from("/formatted").join(dir.file_name().unwrap())));

        mock_formatter.expect_format_file().returning(|_, file| {
            Ok(PathBuf::from(format!(
                "formatted_{}",
                file.to_string_lossy()
            )))
        });

        let converter =
            CommandConverter::with_formatter(EntryFormat::default(), Box::new(mock_formatter));

        let compiler_cmd = CompilerCommand::from_strings(
            "/original/dir",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let entries = converter.to_entries(&command);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].directory, PathBuf::from("/formatted/dir"));
        assert_eq!(entries[0].file, PathBuf::from("formatted_main.c"));
    }

    #[test]
    fn test_path_formatting_error_handling() {
        let mut mock_formatter = MockPathFormatter::new();

        // Make format_directory fail
        mock_formatter.expect_format_directory().returning(|_, _| {
            Err(FormatError::PathCanonicalize(io::Error::new(
                io::ErrorKind::NotFound,
                "Directory not found",
            )))
        });

        let converter =
            CommandConverter::with_formatter(EntryFormat::default(), Box::new(mock_formatter));

        let compiler_cmd = CompilerCommand::from_strings(
            "/nonexistent/dir",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["main.c"])],
        );
        let command = Command::Compiler(compiler_cmd);

        // Should return empty vector when path formatting fails
        let entries = converter.to_entries(&command);
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_file_path_formatting_error_handling() {
        let mut mock_formatter = MockPathFormatter::new();

        // Directory formatting succeeds
        mock_formatter
            .expect_format_directory()
            .returning(|_, dir| Ok(dir.to_path_buf()));

        // File formatting fails
        mock_formatter.expect_format_file().returning(|_, _| {
            Err(FormatError::PathCanonicalize(io::Error::new(
                io::ErrorKind::NotFound,
                "File not found",
            )))
        });

        let converter =
            CommandConverter::with_formatter(EntryFormat::default(), Box::new(mock_formatter));

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["nonexistent.c"])],
        );
        let command = Command::Compiler(compiler_cmd);

        let entries = converter.to_entries(&command);

        // Should still create entry but with original paths (fallback behavior)
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file, PathBuf::from("nonexistent.c"));
        assert_eq!(entries[0].directory, PathBuf::from("/home/user"));
    }

    #[test]
    fn test_output_file_formatting_error_handling() {
        let mut mock_formatter = MockPathFormatter::new();

        // Directory formatting succeeds
        mock_formatter
            .expect_format_directory()
            .returning(|_, dir| Ok(dir.to_path_buf()));

        // File formatting fails for output but succeeds for source
        mock_formatter
            .expect_format_file()
            .withf(|_, path| path.to_string_lossy().contains("main.o"))
            .returning(|_, _| {
                Err(FormatError::PathCanonicalize(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Output file path error",
                )))
            });

        mock_formatter
            .expect_format_file()
            .withf(|_, path| path.to_string_lossy().contains("main.c"))
            .returning(|_, file| Ok(file.to_path_buf()));

        let converter = CommandConverter::with_formatter(
            EntryFormat {
                keep_output_field: true,
                command_field_as_array: true,
            },
            Box::new(mock_formatter),
        );

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let entries = converter.to_entries(&command);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file, PathBuf::from("main.c"));
        // Output should still be present but with original path due to error fallback
        assert_eq!(entries[0].output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn test_configuration_validation_failure() {
        use crate::config::{PathFormat, PathResolver};

        let invalid_format = Format {
            paths: PathFormat {
                directory: PathResolver::Relative,
                file: PathResolver::Absolute,
            },
            entry: EntryFormat::default(),
        };

        let result = CommandConverter::new(invalid_format);
        assert!(result.is_err());
    }
}

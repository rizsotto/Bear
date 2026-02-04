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
//! - Filtering out commands that should not generate compilation database entries
//!
//! # Compilation Database Entry Generation Rules
//!
//! The converter applies specific rules to determine when compilation database entries
//! should be generated:
//!
//! ## Cases that generate NO entries:
//! 1. **Preprocessing-only commands**: Commands with `PassEffect::StopsAt(Preprocessing)`
//! 2. **Info-only commands**: Commands with `PassEffect::InfoAndExit` (e.g., `--version`, `--help`)
//! 3. **Linking-only commands**: Commands without compilation flags and no compilable source files
//! 4. **Commands without source files**: Any command that has no source files to process
//!
//! ## Cases that generate entries:
//! 1. **Compilation commands**: Commands with `PassEffect::StopsAt(Compiling)` or `PassEffect::StopsAt(Assembling)`
//! 2. **Compile-and-link commands**: Commands that both compile and link in one step
//!    - Linking-specific flags (classified as `PassEffect::Configures(Linking)`) are filtered out from entries
//!    - Only compilation-relevant flags are included in the database
//!
//! The converter relies on semantic analysis performed by compiler interpreters to properly
//! classify command-line arguments instead of checking raw flag strings.
//!
//! # Example
//!
//! ```rust
//! use bear::output::clang::converter::CommandConverter;
//! use bear::config::Format;
//!
//! let config = Format::default();
//! let converter = CommandConverter::new(config);
//!
//! // The converter can be used to convert semantic Command instances
//! // into compilation database entries based on the configured format
//! ```

use super::Entry;
use super::{ConfigurablePathFormatter, PathFormatter};
use crate::config;
use crate::semantic::{ArgumentKind, Arguments, Command, CompilerCommand, CompilerPass, PassEffect};
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
    pub fn new(format: config::Format) -> Self {
        let path_formatter = Box::new(ConfigurablePathFormatter::new(format.paths));
        Self { format: format.entries, path_formatter }
    }

    /// Creates a new CommandConverter with a custom path formatter for testing.
    #[cfg(test)]
    pub fn with_formatter(format: config::EntryFormat, path_formatter: Box<dyn PathFormatter>) -> Self {
        Self { format, path_formatter }
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
        // Check if we should skip entry generation for this command
        if self.should_skip_entry_generation(cmd) {
            return vec![];
        }

        // Format working directory
        let Some(formatted_directory) = self.format_working_directory(&cmd.working_dir) else {
            return vec![];
        };

        // Create output file if needed
        let output_file = self.create_output_file(&formatted_directory, &cmd.arguments);

        // Create one entry per source argument (only non-binary source files)
        cmd.arguments
            .iter()
            .filter(|arg| matches!(arg.kind(), ArgumentKind::Source { binary: false }))
            .filter_map(|source_arg| {
                // Get and format source file
                let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
                let source_file_path = source_arg.as_file(path_updater)?;
                let formatted_source_file = self.format_source_file(&formatted_directory, &source_file_path);

                let command_args =
                    self.build_command_args_for_source(cmd, source_arg.as_ref(), &formatted_directory);

                if self.format.use_array_format {
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

    /// Formats the working directory path.
    ///
    /// Returns `Some(formatted_path)` on success, `None` on formatting error.
    fn format_working_directory(&self, working_dir: &Path) -> Option<PathBuf> {
        match self.path_formatter.format_directory(working_dir, working_dir) {
            Ok(dir) => Some(dir),
            Err(e) => {
                warn!("Failed to format directory path: {}", e);
                None
            }
        }
    }

    /// Creates the output file path if the format includes output fields.
    ///
    /// Returns `Some(output_path)` if output should be included and found, `None` otherwise.
    fn create_output_file(
        &self,
        formatted_directory: &Path,
        arguments: &[Box<dyn Arguments>],
    ) -> Option<PathBuf> {
        if !self.format.include_output_field {
            return None;
        }

        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        let output_path = arguments
            .iter()
            .filter(|arg| matches!(arg.kind(), ArgumentKind::Output))
            .nth(0)
            .and_then(|arg| arg.as_file(path_updater))?;

        match self.path_formatter.format_file(formatted_directory, &output_path) {
            Ok(formatted_path) => Some(formatted_path),
            Err(e) => {
                warn!("Failed to format output file path {}: {}", output_path.display(), e);
                Some(output_path)
            }
        }
    }

    /// Formats a source file path.
    ///
    /// Returns the formatted path, falling back to the original path on error.
    fn format_source_file(&self, formatted_directory: &Path, source_file_path: &Path) -> PathBuf {
        match self.path_formatter.format_file(formatted_directory, source_file_path) {
            Ok(formatted_path) => formatted_path,
            Err(e) => {
                warn!("Failed to format source file path {}: {}", source_file_path.display(), e);
                source_file_path.to_path_buf()
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
        let mut command_args = vec![];

        // Add all non-source arguments, while handling source file placement
        for arg in &cmd.arguments {
            // Skip this specific source argument (using pointer equality)
            if matches!(arg.kind(), ArgumentKind::Source { .. }) && !std::ptr::eq(arg.as_ref(), source_arg) {
                continue;
            }

            // Filter out linking-specific arguments for compilation database entries
            if matches!(
                arg.kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
                    | ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Linking))
            ) {
                continue;
            }

            // Get arguments with original paths, then format any file paths
            let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
            let original_args = arg.as_arguments(path_updater);

            // For file-type arguments, we need to format the paths
            match arg.kind() {
                ArgumentKind::Source { .. } | ArgumentKind::Output => {
                    // These might contain file paths that need formatting
                    let formatted_args = original_args
                        .into_iter()
                        .map(|arg_str| {
                            let path = Path::new(&arg_str);
                            if path.is_absolute() || path.extension().is_some() {
                                // Likely a file path, format it
                                self.format_source_file(formatted_directory, path)
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
                ArgumentKind::Compiler => {
                    if let Some(executable_name) = cmd.executable.file_name() {
                        if let Some(name_str) = executable_name.to_str() {
                            command_args.push(name_str.to_string());
                        } else {
                            command_args.extend(original_args);
                        }
                    } else {
                        command_args.extend(original_args);
                    }
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
    /// For `ArgumentKind::Source`, this matches any source regardless of the `binary` flag.
    fn find_arguments_by_kind(
        cmd: &CompilerCommand,
        kind: ArgumentKind,
    ) -> impl Iterator<Item = &Box<dyn Arguments>> {
        cmd.arguments.iter().filter(move |arg| {
            match (arg.kind(), kind) {
                // For Source, match any source regardless of binary flag
                (ArgumentKind::Source { .. }, ArgumentKind::Source { .. }) => true,
                // For other kinds, use exact equality
                (a, b) => a == b,
            }
        })
    }

    /// Determines if we should skip generating compilation database entries for a command.
    ///
    /// Returns true if the command should not generate entries for any of these reasons:
    /// 1. Preprocessing-only commands (`PassEffect::StopsAt(Preprocessing)`)
    /// 2. Info-only commands (`PassEffect::InfoAndExit`)
    /// 3. Commands without source files
    /// 4. Linking-only commands (no compilation flags and has source files)
    fn should_skip_entry_generation(&self, cmd: &CompilerCommand) -> bool {
        // Check if this is an info-only command (e.g., --version, --help)
        if self.is_info_only(cmd) {
            return true;
        }

        // Check if this is a preprocessing-only command (e.g., -E)
        if self.is_preprocessing_only(cmd) {
            return true;
        }

        // Find all source arguments (using binary: false as a placeholder, find_arguments_by_kind matches any source)
        let source_arguments = Self::find_arguments_by_kind(cmd, ArgumentKind::Source { binary: false })
            .collect::<Vec<&Box<dyn Arguments>>>();

        // If no source files found, skip entry generation
        if source_arguments.is_empty() {
            return true;
        }

        // Check if this is a linking-only command
        if self.is_linking_only(cmd) {
            return true;
        }

        false
    }

    /// Determines if a compiler command is preprocessing-only.
    ///
    /// A command is considered preprocessing-only if it has a `PassEffect::StopsAt(Preprocessing)` flag.
    /// This is the `-E` flag which explicitly stops the compiler after preprocessing.
    fn is_preprocessing_only(&self, cmd: &CompilerCommand) -> bool {
        cmd.arguments.iter().any(|arg| {
            matches!(arg.kind(), ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)))
        })
    }

    /// Determines if a compiler command is info-only.
    ///
    /// A command is considered info-only if it contains arguments
    /// classified as `PassEffect::InfoAndExit` by the semantic analysis.
    /// These commands typically display information and don't perform compilation.
    fn is_info_only(&self, cmd: &CompilerCommand) -> bool {
        cmd.arguments.iter().any(|arg| matches!(arg.kind(), ArgumentKind::Other(PassEffect::InfoAndExit)))
    }

    /// Determines if a compiler command is linking-only.
    ///
    /// A command is considered linking-only if:
    /// 1. It does NOT have a `PassEffect::StopsAt(Compiling)` or `PassEffect::StopsAt(Assembling)` flag
    /// 2. AND it has no compilable source files (only object files, libraries, etc.)
    ///
    /// This typically happens when linking pre-compiled object files or libraries.
    ///
    /// The `binary` flag on `ArgumentKind::Source` is set during semantic analysis
    /// by the interpreter, so we can simply check it here.
    fn is_linking_only(&self, cmd: &CompilerCommand) -> bool {
        // Check if the command has a flag that stops before linking (-c or -S)
        let stops_before_linking = cmd.arguments.iter().any(|arg| {
            matches!(
                arg.kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
                    | ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Assembling))
            )
        });

        // If there's a -c or -S flag, it's not linking-only
        if stops_before_linking {
            return false;
        }

        // Check if there are any compilable source files (not binary files)
        // The binary flag is set by the interpreter during semantic analysis
        let has_compilable_sources =
            cmd.arguments.iter().any(|arg| matches!(arg.kind(), ArgumentKind::Source { binary: false }));

        // If no -c/-S flag and no compilable sources, it's linking-only
        !has_compilable_sources
    }
}

#[cfg(test)]
mod tests {
    use super::super::format::{FormatError, MockPathFormatter};
    use super::*;
    use crate::config::{EntryFormat, Format, PathFormat};
    use crate::semantic::{ArgumentKind, Command, CompilerCommand, CompilerPass, PassEffect};
    use std::io;

    #[test]
    fn test_compiler_command_to_entries_single_source() {
        let command = Command::Compiler(CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Other(PassEffect::None), vec!["-Wall"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        ));

        let format = Format { paths: PathFormat::default(), entries: EntryFormat::default() };
        let converter = CommandConverter::new(format);
        let entries = converter.to_entries(&command);

        let expected = vec![Entry::from_arguments_str(
            "main.c",
            vec!["gcc", "-c", "-Wall", "main.c", "-o", "main.o"],
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
                (ArgumentKind::Compiler, vec!["/usr/bin/g++"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["file1.cpp"]),
                (ArgumentKind::Source { binary: false }, vec!["file2.cpp"]),
            ],
        ));

        let format = Format { paths: PathFormat::default(), entries: EntryFormat::default() };
        let converter = CommandConverter::new(format);
        let result = converter.to_entries(&command);

        let expected = vec![
            Entry::from_arguments_str("file1.cpp", vec!["g++", "-c", "file1.cpp"], "/home/user", None),
            Entry::from_arguments_str("file2.cpp", vec!["g++", "-c", "file2.cpp"], "/home/user", None),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compiler_command_to_entries_no_sources() {
        let command = Command::Compiler(CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![(ArgumentKind::Other(PassEffect::InfoAndExit), vec!["--version"])],
        ));

        let format = Format { paths: PathFormat::default(), entries: EntryFormat::default() };
        let converter = CommandConverter::new(format);
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
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        ));
        let format = Format {
            paths: PathFormat::default(),
            entries: EntryFormat { include_output_field: true, use_array_format: false },
        };
        let converter = CommandConverter::new(format);
        let entries = converter.to_entries(&command);

        let expected =
            vec![Entry::from_command_str("main.c", "gcc -c main.c -o main.o", "/home/user", Some("main.o"))];
        assert_eq!(entries, expected);
    }

    #[test]
    fn test_to_entries_without_output_field() {
        let command = Command::Compiler(CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        ));
        let format = Format {
            paths: PathFormat::default(),
            entries: EntryFormat { use_array_format: true, include_output_field: false },
        };
        let sut = CommandConverter::new(format);
        let result = sut.to_entries(&command);

        let expected = vec![Entry::from_arguments_str(
            "main.c",
            vec!["gcc", "-c", "main.c", "-o", "main.o"],
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
            entries: EntryFormat { use_array_format: true, include_output_field: false },
        };
        let converter = CommandConverter::new(format);

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["test.c"]),
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

        mock_formatter
            .expect_format_file()
            .returning(|_, file| Ok(PathBuf::from(format!("formatted_{}", file.to_string_lossy()))));

        let converter = CommandConverter::with_formatter(EntryFormat::default(), Box::new(mock_formatter));

        let compiler_cmd = CompilerCommand::from_strings(
            "/original/dir",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
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
            Err(FormatError::PathCanonicalize(io::Error::new(io::ErrorKind::NotFound, "Directory not found")))
        });

        let converter = CommandConverter::with_formatter(EntryFormat::default(), Box::new(mock_formatter));

        let compiler_cmd = CompilerCommand::from_strings(
            "/nonexistent/dir",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["main.c"])],
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
        mock_formatter.expect_format_directory().returning(|_, dir| Ok(dir.to_path_buf()));

        // File formatting fails
        mock_formatter.expect_format_file().returning(|_, _| {
            Err(FormatError::PathCanonicalize(io::Error::new(io::ErrorKind::NotFound, "File not found")))
        });

        let converter = CommandConverter::with_formatter(EntryFormat::default(), Box::new(mock_formatter));

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["nonexistent.c"])],
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
        mock_formatter.expect_format_directory().returning(|_, dir| Ok(dir.to_path_buf()));

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
            EntryFormat { include_output_field: true, use_array_format: true },
            Box::new(mock_formatter),
        );

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
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
    fn test_preprocessing_only_command_no_entries() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)), vec!["-E"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_linking_only_command_no_entries() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        // Linking object files (no -c flag, object file inputs)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: true }, vec!["main.o"]),
                (ArgumentKind::Source { binary: true }, vec!["lib.o"]),
                (ArgumentKind::Output, vec!["-o", "program"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-L/usr/lib"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_compile_only_command_generates_entries() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, PathBuf::from("main.c"));
    }

    #[test]
    fn test_compile_and_link_filters_linking_flags() {
        let format = Format {
            paths: PathFormat::default(),
            entries: EntryFormat { use_array_format: true, include_output_field: false },
        };
        let converter = CommandConverter::new(format);

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-L/usr/lib"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-lmath"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)), vec!["-Wall"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 1);

        let entry = &result[0];
        assert_eq!(entry.file, PathBuf::from("main.c"));

        // Check that linking flags are filtered out
        let args_str = entry.arguments.join(" ");
        assert!(args_str.contains("-Wall")); // Compile flag should be present
        assert!(!args_str.contains("-L/usr/lib")); // Link flag should be filtered
        assert!(!args_str.contains("-lmath")); // Link flag should be filtered
    }

    #[test]
    fn test_info_command_no_entries() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![(ArgumentKind::Other(PassEffect::InfoAndExit), vec!["--version"])],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_realistic_source_file_detection() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        // Test compile-and-link with real source files (should generate entries)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]), // Real source file
                (ArgumentKind::Source { binary: false }, vec!["utils.cpp"]), // Real source file
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-lm"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 2); // Should generate entries for both source files

        // Test linking with object files only (should not generate entries)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: true }, vec!["main.o"]), // Object file
                (ArgumentKind::Source { binary: true }, vec!["utils.a"]), // Static library
                (ArgumentKind::Output, vec!["-o", "program"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 0); // Should not generate entries for object files
    }

    #[test]
    fn test_semantic_classification_vs_raw_flags() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        // Test that we rely on semantic classification, not raw flag strings
        // This tests a hypothetical case where a flag might look like "-E" but
        // is classified differently by semantic analysis

        // Test preprocessing flag properly classified
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (
                    ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
                    vec!["-E"], // Semantically classified as preprocessing
                ),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);
        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 0); // Should skip preprocessing commands

        // Test compilation flag properly classified
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (
                    ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
                    vec!["-c"], // Semantically classified as compiling
                ),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);
        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 1); // Should generate entry for compilation

        // Test info flag properly classified
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![(
                ArgumentKind::Other(PassEffect::InfoAndExit),
                vec!["--version"], // Semantically classified as info
            )],
        );
        let command = Command::Compiler(compiler_cmd);
        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 0); // Should skip info commands

        // Test that linking flags are filtered out (not raw string matching)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
                    vec!["-lmath"], // Semantically classified as linking
                ),
                (
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
                    vec!["-O2"], // Compilation flag
                ),
            ],
        );
        let command = Command::Compiler(compiler_cmd);
        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 1);

        // Verify linking flag is filtered out while compilation flag remains
        let args_str = result[0].arguments.join(" ");
        assert!(!args_str.contains("-lmath")); // Linking flag filtered
        assert!(args_str.contains("-O2")); // Compilation flag preserved
    }

    #[test]
    fn test_consistent_formatting_methods() {
        let format = Format {
            paths: PathFormat::default(),
            entries: EntryFormat { use_array_format: true, include_output_field: true },
        };
        let converter = CommandConverter::new(format);

        // Test that all three formatting methods work consistently
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Compiler, vec!["gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);
        assert_eq!(result.len(), 1);

        let entry = &result[0];

        // Verify all three formatting methods produced results:
        // 1. Working directory formatting
        assert_eq!(entry.directory, PathBuf::from("/home/user"));

        // 2. Source file formatting
        assert_eq!(entry.file, PathBuf::from("main.c"));

        // 3. Output file formatting
        assert_eq!(entry.output, Some(PathBuf::from("main.o")));

        // Verify the command includes the formatted paths
        assert!(entry.arguments.contains(&"gcc".to_string()));
        assert!(entry.arguments.contains(&"-c".to_string()));
        assert!(entry.arguments.contains(&"main.c".to_string()));
        assert!(entry.arguments.contains(&"-o".to_string()));
        assert!(entry.arguments.contains(&"main.o".to_string()));
    }

    #[test]
    fn test_preprocessing_and_compilation_flags_generates_entries() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        // Test command with both preprocessing flags (-D) and compilation flags (-c)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Compiler, vec!["gcc"]),
                (
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
                    vec!["-DWRAPPER_FLAG"],
                ),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["test.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);

        // Should generate entries because it has compilation flags, not just preprocessing
        assert_eq!(result.len(), 1);

        let entry = &result[0];
        assert_eq!(entry.file, PathBuf::from("test.c"));
        assert_eq!(entry.directory, PathBuf::from("/home/user"));

        // Verify the arguments include both preprocessing and compilation flags
        assert!(entry.arguments.contains(&"gcc".to_string()));
        assert!(entry.arguments.contains(&"-DWRAPPER_FLAG".to_string()));
        assert!(entry.arguments.contains(&"-c".to_string()));
        assert!(entry.arguments.contains(&"test.c".to_string()));
    }

    #[test]
    fn test_preprocessing_only_with_defines_no_entries() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        // Test command with only preprocessing flags (no -c flag)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Compiler, vec!["gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)), vec!["-E"]),
                (
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
                    vec!["-DSOME_DEFINE"],
                ),
                (ArgumentKind::Source { binary: false }, vec!["test.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);

        // Should NOT generate entries because it's preprocessing-only (has -E flag)
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_driver_option_does_not_affect_entry_generation() {
        let format = Format::default();
        let converter = CommandConverter::new(format);

        // Test command with driver options like -pipe (should still generate entries)
        let compiler_cmd = CompilerCommand::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Compiler, vec!["gcc"]),
                (ArgumentKind::Other(PassEffect::DriverOption), vec!["-pipe"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );
        let command = Command::Compiler(compiler_cmd);

        let result = converter.to_entries(&command);

        // Should generate entries - driver options don't stop compilation
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, PathBuf::from("main.c"));

        // Verify -pipe is included in the command
        assert!(result[0].arguments.contains(&"-pipe".to_string()));
    }
}

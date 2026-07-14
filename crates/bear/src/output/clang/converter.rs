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

use super::Entry;
use super::path_format::{ResolveFn, resolver_for};
use crate::config;
use crate::semantic::{Argument, ArgumentKind, Command, CompilerPass, PassEffect, SourceMode};
use log::warn;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Converts commands into compilation database entries.
///
/// This converter takes format configuration during construction and uses it
/// to convert commands into appropriately formatted entries.
pub struct CommandConverter {
    format: config::EntryFormat,
    directory_resolver: ResolveFn,
    file_resolver: ResolveFn,
}

impl CommandConverter {
    /// Creates a new CommandConverter with the specified format configuration.
    pub fn new(format: config::Format) -> Self {
        Self::from_resolvers(
            format.entries,
            resolver_for(format.paths.directory),
            resolver_for(format.paths.file),
        )
    }

    /// Constructs the converter from raw resolver functions. Production code
    /// goes through `new`; tests use this directly to inject deterministic
    /// resolvers without touching the filesystem.
    fn from_resolvers(
        format: config::EntryFormat,
        directory_resolver: ResolveFn,
        file_resolver: ResolveFn,
    ) -> Self {
        Self { format, directory_resolver, file_resolver }
    }

    /// Converts a compiler command into compilation database entries.
    pub fn convert(&self, cmd: &Command) -> Vec<Entry> {
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

        match cmd.source_mode {
            SourceMode::PerSourceStripped => {
                // Create one entry per source argument (only non-binary source files),
                // each entry keeping only its own source (siblings stripped).
                cmd.arguments
                    .iter()
                    .enumerate()
                    .filter(|(_, arg)| matches!(arg.kind(), ArgumentKind::Source { binary: false }))
                    .filter_map(|(source_idx, source_arg)| {
                        self.build_entry(
                            cmd,
                            source_arg,
                            Some(source_idx),
                            &formatted_directory,
                            &output_file,
                        )
                    })
                    .collect()
            }
            SourceMode::PerSourceFull => {
                // Whole-module compiler (e.g. swiftc): one entry per compilable
                // source, but every entry keeps the complete invocation (no
                // sibling stripping), because each file's semantics depend on
                // every other source in the same invocation.
                cmd.arguments
                    .iter()
                    .filter(|arg| matches!(arg.kind(), ArgumentKind::Source { binary: false }))
                    .filter_map(|source_arg| {
                        self.build_entry(cmd, source_arg, None, &formatted_directory, &output_file)
                    })
                    .collect()
            }
            SourceMode::Combined => {
                // Single-translation-unit compiler (e.g. valac): the whole invocation
                // collapses to one entry whose `file` is the first compilable source.
                // All sources are kept in the arguments (no sibling stripping).
                let first_source = cmd
                    .arguments
                    .iter()
                    .find(|arg| matches!(arg.kind(), ArgumentKind::Source { binary: false }));
                let Some(source_arg) = first_source else {
                    // No compilable source; should_skip_entry_generation already
                    // covers this, but keep the guard local to the branch.
                    return vec![];
                };

                self.build_entry(cmd, source_arg, None, &formatted_directory, &output_file)
                    .into_iter()
                    .collect()
            }
        }
    }

    /// Builds a single compilation database entry for `source_arg`.
    ///
    /// `selected` controls which sources are kept in the arguments:
    /// `Some(idx)` keeps only the source at that index
    /// (`SourceMode::PerSourceStripped`, one entry per source with siblings
    /// stripped). `None` keeps every source -- used both for
    /// `SourceMode::Combined` (one entry total) and `SourceMode::PerSourceFull`
    /// (one entry per source, but each keeps the full invocation).
    fn build_entry(
        &self,
        cmd: &Command,
        source_arg: &Argument,
        selected: Option<usize>,
        formatted_directory: &Path,
        output_file: &Option<PathBuf>,
    ) -> Option<Entry> {
        // Get and format source file
        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        let source_file_path = source_arg.as_file(path_updater)?;
        let formatted_source_file = self.format_source_file(formatted_directory, &source_file_path);

        let command_args = self.build_command_args(cmd, selected, formatted_directory);

        if self.format.use_array_format {
            Some(Entry::with_arguments(
                formatted_source_file,
                command_args,
                formatted_directory,
                output_file.as_ref(),
            ))
        } else {
            Some(Entry::with_command(
                formatted_source_file,
                command_args,
                formatted_directory,
                output_file.as_ref(),
            ))
        }
    }

    /// Formats the working directory path.
    ///
    /// Returns `Some(formatted_path)` on success, `None` on formatting error.
    fn format_working_directory(&self, working_dir: &Path) -> Option<PathBuf> {
        match (self.directory_resolver)(working_dir, working_dir) {
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
    fn create_output_file(&self, formatted_directory: &Path, arguments: &[Argument]) -> Option<PathBuf> {
        if !self.format.include_output_field {
            return None;
        }

        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        let output_path = arguments
            .iter()
            .filter(|arg| matches!(arg.kind(), ArgumentKind::Output))
            .nth(0)
            .and_then(|arg| arg.as_file(path_updater))?;

        match (self.file_resolver)(formatted_directory, &output_path) {
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
        match (self.file_resolver)(formatted_directory, source_file_path) {
            Ok(formatted_path) => formatted_path,
            Err(e) => {
                warn!("Failed to format source file path {}: {}", source_file_path.display(), e);
                source_file_path.to_path_buf()
            }
        }
    }

    /// Builds command arguments for an entry.
    ///
    /// This method constructs the command arguments list that includes the executable,
    /// all non-source arguments, and the selected source file(s).
    /// It ensures that source files are placed in the correct position relative to output arguments.
    ///
    /// `selected` is `Some(idx)` for `SourceMode::PerSourceStripped` (keep only
    /// the source at `idx`, strip the siblings) or `None` for
    /// `SourceMode::Combined`/`SourceMode::PerSourceFull` (keep every source).
    fn build_command_args(
        &self,
        cmd: &Command,
        selected: Option<usize>,
        formatted_directory: &Path,
    ) -> Vec<String> {
        let mut command_args = vec![];

        for (idx, arg) in cmd.arguments.iter().enumerate() {
            // For separable compilers, skip the other source arguments (keep only
            // the one we are building for). For combined compilers (`None`), keep all.
            if let Some(source_arg_idx) = selected
                && matches!(arg.kind(), ArgumentKind::Source { .. })
                && idx != source_arg_idx
            {
                continue;
            }

            // Filter out linking-specific arguments for compilation database entries
            if matches!(
                arg.kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
                    | ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Linking))
                    | ArgumentKind::Other(PassEffect::PassThrough)
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
                    if let Some(exe_str) = cmd.executable.to_str() {
                        command_args.push(exe_str.to_string());
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
    fn find_arguments_by_kind(cmd: &Command, kind: ArgumentKind) -> impl Iterator<Item = &Argument> {
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
    fn should_skip_entry_generation(&self, cmd: &Command) -> bool {
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
            .collect::<Vec<&Argument>>();

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
    fn is_preprocessing_only(&self, cmd: &Command) -> bool {
        cmd.arguments.iter().any(|arg| {
            matches!(arg.kind(), ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)))
        })
    }

    /// Determines if a compiler command is info-only.
    ///
    /// A command is considered info-only if it contains arguments
    /// classified as `PassEffect::InfoAndExit` by the semantic analysis.
    /// These commands typically display information and don't perform compilation.
    fn is_info_only(&self, cmd: &Command) -> bool {
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
    fn is_linking_only(&self, cmd: &Command) -> bool {
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
    use super::super::path_format::FormatError;
    use super::*;
    use crate::config::{ArgumentsFormat, EntryFormat, Format, PathFormat, PathResolver};
    use crate::semantic::{ArgumentKind, Command, CompilerPass, PassEffect};
    use std::ffi::OsStr;
    use std::io;

    fn resolver_identity(_base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
        Ok(path.to_path_buf())
    }

    fn resolver_always_fails(_base: &Path, _path: &Path) -> Result<PathBuf, FormatError> {
        Err(FormatError::PathCanonicalize(io::Error::other("test injected")))
    }

    fn resolver_fail_for_object_files(_base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
        if path.extension() == Some(OsStr::new("o")) {
            Err(FormatError::PathCanonicalize(io::Error::other("test injected")))
        } else {
            Ok(path.to_path_buf())
        }
    }

    #[test]
    fn test_compiler_command_to_entries_single_source() {
        let command = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Other(PassEffect::None), vec!["-Wall"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let result = sut.convert(&command);

        let expected = vec![Entry::from_arguments_str(
            "main.c",
            vec!["/usr/bin/gcc", "-c", "-Wall", "main.c", "-o", "main.o"],
            "/home/user",
            Some("main.o"),
        )];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_compiler_command_to_entries_multiple_sources() {
        let command = Command::from_strings(
            "/home/user",
            "/usr/bin/g++",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/g++"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["file1.cpp"]),
                (ArgumentKind::Source { binary: false }, vec!["file2.cpp"]),
            ],
        );

        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let result = sut.convert(&command);

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
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![(ArgumentKind::Other(PassEffect::InfoAndExit), vec!["--version"])],
        );

        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let result = sut.convert(&command);

        let expected: Vec<Entry> = vec![];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_to_entries_command_field_as_string() {
        let command = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { include_output_field: true, use_array_format: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let result = sut.convert(&command);

        let expected = vec![Entry::from_command_str(
            "main.c",
            "/usr/bin/gcc -c main.c -o main.o",
            "/home/user",
            Some("main.o"),
        )];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_to_entries_without_output_field() {
        let command = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let result = sut.convert(&command);

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
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["test.c"]),
            ],
        );

        let result = sut.convert(&command);

        assert_eq!(result.len(), 1);
        // Verify the entry is valid using the public API
        assert!(result[0].validate().is_ok());
    }

    #[test]
    fn test_directory_format_failure_yields_no_entries() {
        let sut = CommandConverter::from_resolvers(
            EntryFormat::default(),
            resolver_always_fails,
            resolver_identity,
        );
        let cmd = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["main.c"])],
        );
        assert!(sut.convert(&cmd).is_empty());
    }

    #[test]
    fn test_file_format_failure_falls_back_to_original_path() {
        let sut = CommandConverter::from_resolvers(
            EntryFormat::default(),
            resolver_identity,
            resolver_always_fails,
        );
        let cmd = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["nonexistent.c"])],
        );

        let entries = sut.convert(&cmd);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].directory, PathBuf::from("/home/user"));
        assert_eq!(entries[0].file, PathBuf::from("nonexistent.c"));
    }

    #[test]
    fn test_output_format_failure_falls_back_only_for_output() {
        let sut = CommandConverter::from_resolvers(
            EntryFormat { include_output_field: true, use_array_format: true },
            resolver_identity,
            resolver_fail_for_object_files,
        );
        let cmd = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let entries = sut.convert(&cmd);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file, PathBuf::from("main.c"));
        assert_eq!(entries[0].output, Some(PathBuf::from("main.o")));
    }

    #[test]
    fn test_preprocessing_only_command_no_entries() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)), vec!["-E"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );

        let result = sut.convert(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_linking_only_command_no_entries() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Linking object files (no -c flag, object file inputs)
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: true }, vec!["main.o"]),
                (ArgumentKind::Source { binary: true }, vec!["lib.o"]),
                (ArgumentKind::Output, vec!["-o", "program"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-L/usr/lib"]),
            ],
        );

        let result = sut.convert(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_compile_only_command_generates_entries() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let result = sut.convert(&command);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, PathBuf::from("main.c"));
    }

    #[test]
    fn test_compile_and_link_filters_linking_flags() {
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-L/usr/lib"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-lmath"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)), vec!["-Wall"]),
            ],
        );

        let result = sut.convert(&command);
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
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![(ArgumentKind::Other(PassEffect::InfoAndExit), vec!["--version"])],
        );

        let result = sut.convert(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_realistic_source_file_detection() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Test compile-and-link with real source files (should generate entries)
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]), // Real source file
                (ArgumentKind::Source { binary: false }, vec!["utils.cpp"]), // Real source file
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-lm"]),
            ],
        );

        let result = sut.convert(&command);
        assert_eq!(result.len(), 2); // Should generate entries for both source files

        // Test linking with object files only (should not generate entries)
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Source { binary: true }, vec!["main.o"]), // Object file
                (ArgumentKind::Source { binary: true }, vec!["utils.a"]), // Static library
                (ArgumentKind::Output, vec!["-o", "program"]),
            ],
        );

        let result = sut.convert(&command);
        assert_eq!(result.len(), 0); // Should not generate entries for object files
    }

    #[test]
    fn test_semantic_classification_vs_raw_flags() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Test that we rely on semantic classification, not raw flag strings
        // This tests a hypothetical case where a flag might look like "-E" but
        // is classified differently by semantic analysis

        // Test preprocessing flag properly classified
        let command = Command::from_strings(
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
        let result = sut.convert(&command);
        assert_eq!(result.len(), 0); // Should skip preprocessing commands

        // Test compilation flag properly classified
        let command = Command::from_strings(
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
        let result = sut.convert(&command);
        assert_eq!(result.len(), 1); // Should generate entry for compilation

        // Test info flag properly classified
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![(
                ArgumentKind::Other(PassEffect::InfoAndExit),
                vec!["--version"], // Semantically classified as info
            )],
        );
        let result = sut.convert(&command);
        assert_eq!(result.len(), 0); // Should skip info commands

        // Test that linking flags are filtered out (not raw string matching)
        let command = Command::from_strings(
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
        let result = sut.convert(&command);
        assert_eq!(result.len(), 1);

        // Verify linking flag is filtered out while compilation flag remains
        let args_str = result[0].arguments.join(" ");
        assert!(!args_str.contains("-lmath")); // Linking flag filtered
        assert!(args_str.contains("-O2")); // Compilation flag preserved
    }

    #[test]
    fn test_consistent_formatting_methods() {
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: true },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        // Test that all three formatting methods work consistently
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Compiler, vec!["gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let result = sut.convert(&command);
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
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Test command with both preprocessing flags (-D) and compilation flags (-c)
        let command = Command::from_strings(
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

        let result = sut.convert(&command);

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
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Test command with only preprocessing flags (no -c flag)
        let command = Command::from_strings(
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

        let result = sut.convert(&command);

        // Should NOT generate entries because it's preprocessing-only (has -E flag)
        assert_eq!(result.len(), 0);
    }

    // --- Tests for non-trivial PathFormat configurations (end-to-end through CommandConverter::new) ---

    #[test]
    fn test_absolute_path_format_makes_paths_absolute() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();
        let working_dir = temp_path.to_str().unwrap();

        // Create actual source file so canonicalize can work if needed
        std::fs::write(temp_path.join("main.c"), "").unwrap();

        let sut = {
            let format = Format {
                paths: PathFormat { directory: PathResolver::Absolute, file: PathResolver::Absolute },
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            working_dir,
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let entries = sut.convert(&command);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        // Directory should be absolute
        assert!(entry.directory.is_absolute(), "directory should be absolute: {:?}", entry.directory);
        // File should be absolute
        assert!(entry.file.is_absolute(), "file should be absolute: {:?}", entry.file);
    }

    #[test]
    fn test_relative_path_format_makes_paths_relative() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();
        let working_dir = temp_path.to_str().unwrap();

        let source_file = temp_path.join("main.c");
        std::fs::write(&source_file, "").unwrap();

        let sut = {
            let format = Format {
                paths: PathFormat { directory: PathResolver::Relative, file: PathResolver::Relative },
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        // Use a relative source file (relative to working dir)
        let command = Command::from_strings(
            working_dir,
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );

        let entries = sut.convert(&command);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        // Directory should be relative (the working dir resolved relative to itself is ".")
        assert!(!entry.directory.is_absolute(), "directory should be relative: {:?}", entry.directory);
        // File should be relative
        assert!(!entry.file.is_absolute(), "file should be relative: {:?}", entry.file);
    }

    #[test]
    fn test_canonical_path_format_resolves_dotdot() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create subdirectory and file
        let sub_dir = temp_path.join("src");
        std::fs::create_dir(&sub_dir).unwrap();
        let source_file = sub_dir.join("main.c");
        std::fs::write(&source_file, "").unwrap();

        // Use a path with .. components
        let working_dir_with_dotdot = sub_dir.join("..").join("src");
        let working_dir_str = working_dir_with_dotdot.to_str().unwrap();

        let sut = {
            let format = Format {
                paths: PathFormat { directory: PathResolver::Canonical, file: PathResolver::Canonical },
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            working_dir_str,
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec![source_file.to_str().unwrap()]),
            ],
        );

        let entries = sut.convert(&command);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        // Canonical paths should not contain ".."
        let dir_str = entry.directory.to_string_lossy();
        assert!(!dir_str.contains(".."), "canonical directory should not contain '..': {}", dir_str);
        let file_str = entry.file.to_string_lossy();
        assert!(!file_str.contains(".."), "canonical file should not contain '..': {}", file_str);
    }

    #[test]
    fn test_mixed_path_format_absolute_directory_relative_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();
        let working_dir = temp_path.to_str().unwrap();

        std::fs::write(temp_path.join("main.c"), "").unwrap();

        let sut = {
            let format = Format {
                paths: PathFormat { directory: PathResolver::Absolute, file: PathResolver::Relative },
                entries: EntryFormat::default(),
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            working_dir,
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );

        let entries = sut.convert(&command);
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        // Directory should be absolute
        assert!(entry.directory.is_absolute(), "directory should be absolute: {:?}", entry.directory);
        // File should be relative
        assert!(!entry.file.is_absolute(), "file should be relative: {:?}", entry.file);
    }

    /// Builds a `Command` with `source_mode` set to `Combined`, mirroring what
    /// the valac interpreter produces. `from_strings` always sets
    /// `PerSourceStripped`, so combined-path tests flip it here.
    fn combined(mut cmd: Command) -> Command {
        cmd.source_mode = crate::semantic::SourceMode::Combined;
        cmd
    }

    /// Builds a `Command` with `source_mode` set to `PerSourceFull`, mirroring
    /// what the swiftc interpreter produces. `from_strings` always sets
    /// `PerSourceStripped`, so whole-module tests flip it here.
    fn per_source_full(mut cmd: Command) -> Command {
        cmd.source_mode = crate::semantic::SourceMode::PerSourceFull;
        cmd
    }

    // Requirements: output-compilation-entries
    #[test]
    fn test_combined_sources_produce_single_entry() {
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        // valac-style: two sources compiled as one translation unit, producing a
        // library/binding. `--library`/`--vapi` classify as `none`, not stripped.
        let command = combined(Command::from_strings(
            "/home/user",
            "valac",
            vec![
                (ArgumentKind::Compiler, vec!["valac"]),
                (ArgumentKind::Other(PassEffect::None), vec!["--library", "foo"]),
                (ArgumentKind::Source { binary: false }, vec!["a.vala"]),
                (ArgumentKind::Source { binary: false }, vec!["b.vala"]),
                (ArgumentKind::Source { binary: false }, vec!["c.vala"]),
            ],
        ));

        let result = sut.convert(&command);

        // Exactly one entry, file == first source, all three sources retained.
        assert_eq!(result.len(), 1);
        let entry = &result[0];
        assert_eq!(entry.file, PathBuf::from("a.vala"));

        let args_str = entry.arguments.join(" ");
        assert!(args_str.contains("a.vala"), "first source must be present: {}", args_str);
        assert!(args_str.contains("b.vala"), "sibling source must be retained: {}", args_str);
        assert!(args_str.contains("c.vala"), "sibling source must be retained: {}", args_str);
        assert!(args_str.contains("--library"), "non-link flag must be retained: {}", args_str);
        assert!(args_str.contains("foo"), "--library value must be retained: {}", args_str);
    }

    // Requirements: output-compilation-entries
    #[test]
    fn test_combined_sources_emit_command_string_form() {
        // vala-language-server reads only the command-string form, so guard that
        // a combined entry serializes its single command (with every source) when
        // use_array_format is false.
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: false, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = combined(Command::from_strings(
            "/home/user",
            "valac",
            vec![
                (ArgumentKind::Compiler, vec!["valac"]),
                (ArgumentKind::Other(PassEffect::None), vec!["--library", "foo"]),
                (ArgumentKind::Source { binary: false }, vec!["a.vala"]),
                (ArgumentKind::Source { binary: false }, vec!["b.vala"]),
            ],
        ));

        let result = sut.convert(&command);

        // Exactly one entry in command-string form: `command` set, `arguments` empty.
        assert_eq!(result.len(), 1);
        let entry = &result[0];
        assert_eq!(entry.file, PathBuf::from("a.vala"));
        assert!(entry.arguments.is_empty(), "command form must leave arguments empty");
        assert!(entry.command.contains("a.vala"), "command must keep first source: {}", entry.command);
        assert!(entry.command.contains("b.vala"), "command must keep sibling source: {}", entry.command);
        assert!(entry.command.contains("--library"), "command must keep non-link flag: {}", entry.command);
    }

    // Requirements: output-compilation-entries
    #[test]
    fn test_combined_sources_strip_link_only_flags() {
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = combined(Command::from_strings(
            "/home/user",
            "valac",
            vec![
                (ArgumentKind::Compiler, vec!["valac"]),
                (ArgumentKind::Source { binary: false }, vec!["a.vala"]),
                (ArgumentKind::Source { binary: false }, vec!["b.vala"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-L/usr/lib"]),
            ],
        ));

        let result = sut.convert(&command);

        assert_eq!(result.len(), 1);
        let args_str = result[0].arguments.join(" ");
        assert!(args_str.contains("a.vala"));
        assert!(args_str.contains("b.vala"));
        assert!(!args_str.contains("-L/usr/lib"), "link-only flag must be stripped: {}", args_str);
    }

    // Requirements: output-compilation-entries
    #[test]
    fn test_separable_sources_remain_one_entry_per_source() {
        // Regression guard: the default separable path still fans out.
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        let command = Command::from_strings(
            "/home/user",
            "/usr/bin/g++",
            vec![
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["file1.cpp"]),
                (ArgumentKind::Source { binary: false }, vec!["file2.cpp"]),
            ],
        );

        let result = sut.convert(&command);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file, PathBuf::from("file1.cpp"));
        assert_eq!(result[1].file, PathBuf::from("file2.cpp"));
    }

    // Requirements: output-compilation-entries
    #[test]
    fn test_combined_sources_skip_leading_binary_input() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // A binary input appears before the first compilable source: the
        // representative `file` must be the source, and the binary input is not
        // emitted as its own entry.
        let command = combined(Command::from_strings(
            "/home/user",
            "valac",
            vec![
                (ArgumentKind::Compiler, vec!["valac"]),
                (ArgumentKind::Source { binary: true }, vec!["prebuilt.o"]),
                (ArgumentKind::Source { binary: false }, vec!["a.vala"]),
                (ArgumentKind::Source { binary: false }, vec!["b.vala"]),
            ],
        ));

        let result = sut.convert(&command);

        // Exactly one entry: the binary input is not promoted to its own entry,
        // and the representative `file` is the first compilable source.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, PathBuf::from("a.vala"));
        // Combined mode keeps every input in the command, including the binary one
        // (no sibling stripping); only the entry count collapses to one.
        let args_str = result[0].arguments.join(" ");
        assert!(args_str.contains("prebuilt.o"), "binary input must remain in args: {}", args_str);
        assert!(args_str.contains("a.vala"), "first source must be present: {}", args_str);
        assert!(args_str.contains("b.vala"), "sibling source must be present: {}", args_str);
    }

    // Requirements: output-compilation-entries
    #[test]
    fn test_combined_sources_with_no_compilable_source_produce_no_entry() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Only binary inputs: a pure link step yields no entry, same as separable.
        let command = combined(Command::from_strings(
            "/home/user",
            "valac",
            vec![
                (ArgumentKind::Compiler, vec!["valac"]),
                (ArgumentKind::Source { binary: true }, vec!["prebuilt.o"]),
                (ArgumentKind::Output, vec!["-o", "program"]),
            ],
        ));

        let result = sut.convert(&command);

        assert_eq!(result.len(), 0);
    }

    // Requirements: output-compilation-entries, recognition-compiler-names
    #[test]
    fn test_per_source_full_produces_one_entry_per_source_with_full_arguments() {
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        // swiftc-style whole-module invocation: two sources compiled together,
        // each file's semantics depend on the whole module.
        let command = per_source_full(Command::from_strings(
            "/home/user",
            "swiftc",
            vec![
                (ArgumentKind::Compiler, vec!["swiftc"]),
                (ArgumentKind::Other(PassEffect::None), vec!["-module-name", "App"]),
                (ArgumentKind::Source { binary: false }, vec!["a.swift"]),
                (ArgumentKind::Source { binary: false }, vec!["b.swift"]),
            ],
        ));

        let result = sut.convert(&command);

        // Two entries, one per source, each keeping the whole invocation.
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file, PathBuf::from("a.swift"));
        assert_eq!(result[1].file, PathBuf::from("b.swift"));

        for entry in &result {
            let args_str = entry.arguments.join(" ");
            assert!(
                args_str.contains("a.swift"),
                "entry for {:?} must keep a.swift: {}",
                entry.file,
                args_str
            );
            assert!(
                args_str.contains("b.swift"),
                "entry for {:?} must keep b.swift: {}",
                entry.file,
                args_str
            );
            assert!(
                args_str.contains("-module-name"),
                "entry for {:?} must keep non-source flags: {}",
                entry.file,
                args_str
            );
        }
    }

    // Requirements: output-compilation-entries, recognition-compiler-names
    #[test]
    fn test_per_source_full_strips_link_only_flags() {
        let sut = {
            let format = Format {
                paths: PathFormat::default(),
                entries: EntryFormat { use_array_format: true, include_output_field: false },
                arguments: ArgumentsFormat::default(),
            };
            CommandConverter::new(format)
        };

        let command = per_source_full(Command::from_strings(
            "/home/user",
            "swiftc",
            vec![
                (ArgumentKind::Compiler, vec!["swiftc"]),
                (ArgumentKind::Source { binary: false }, vec!["a.swift"]),
                (ArgumentKind::Source { binary: false }, vec!["b.swift"]),
                (ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)), vec!["-lm"]),
            ],
        ));

        let result = sut.convert(&command);

        assert_eq!(result.len(), 2);
        for entry in &result {
            let args_str = entry.arguments.join(" ");
            assert!(!args_str.contains("-lm"), "link-only flag must be stripped: {}", args_str);
        }
    }

    // Requirements: output-compilation-entries, recognition-compiler-names
    #[test]
    fn test_per_source_full_with_no_compilable_source_produces_no_entries() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        let command = per_source_full(Command::from_strings(
            "/home/user",
            "swiftc",
            vec![
                (ArgumentKind::Compiler, vec!["swiftc"]),
                (ArgumentKind::Other(PassEffect::InfoAndExit), vec!["--version"]),
            ],
        ));

        let result = sut.convert(&command);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_driver_option_does_not_affect_entry_generation() {
        let sut = {
            let format = Format::default();
            CommandConverter::new(format)
        };

        // Test command with driver options like -pipe (should still generate entries)
        let command = Command::from_strings(
            "/home/user",
            "gcc",
            vec![
                (ArgumentKind::Compiler, vec!["gcc"]),
                (ArgumentKind::Other(PassEffect::DriverOption), vec!["-pipe"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
            ],
        );

        let result = sut.convert(&command);

        // Should generate entries - driver options don't stop compilation
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, PathBuf::from("main.c"));

        // Verify -pipe is included in the command
        assert!(result[0].arguments.contains(&"-pipe".to_string()));
    }
}

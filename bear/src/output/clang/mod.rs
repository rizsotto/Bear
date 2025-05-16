// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides support for reading and writing JSON compilation database files.
//!
//! A compilation database is a set of records which describe the compilation of the
//! source files in a given project. It describes the compiler invocation command to
//! compile a source module to an object file.
//!
//! This database can have many forms. One well known and supported format is the JSON
//! compilation database, which is a simple JSON file having the list of compilation
//! as an array. The definition of the JSON compilation database files is done in the
//! LLVM project [documentation](https://clang.llvm.org/docs/JSONCompilationDatabase.html).

mod filter_duplicates;
mod formatter;

use super::formats::{FileFormat, JsonCompilationDatabase};
use super::IteratorWriter;
use crate::{config, semantic};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use shell_words;
use std::{fs, io, path};
use thiserror::Error;

/// Represents an entry of the compilation database.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The main translation unit source processed by this compilation step.
    /// This is used by tools as the key into the compilation database.
    /// There can be multiple command objects for the same file, for example if the same
    /// source file is compiled with different configurations.
    pub file: path::PathBuf,
    /// The compile command argv as list of strings. This should run the compilation step
    /// for the translation unit file. `arguments[0]` should be the executable name, such
    /// as `clang++`. Arguments should not be escaped, but ready to pass to `execvp()`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub arguments: Vec<String>,
    /// The compile command as a single shell-escaped string. Arguments may be shell quoted
    /// and escaped following platform conventions, with ‘"’ and ‘\’ being the only special
    /// characters. Shell expansion is not supported.
    ///
    /// Either `arguments` or `command` is required. `arguments` is preferred, as shell
    /// (un)escaping is a possible source of errors.
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub command: String,
    /// The working directory of the compilation. All paths specified in the `command` or
    /// `file` fields must be either absolute or relative to this directory.
    pub directory: path::PathBuf,
    /// The name of the output created by this compilation step. This field is optional.
    /// It can be used to distinguish different processing modes of the same input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub output: Option<path::PathBuf>,
}

impl Entry {
    /// Create an Entry from arguments (preferred).
    pub fn from_arguments(
        file: impl Into<path::PathBuf>,
        arguments: Vec<String>,
        directory: impl Into<path::PathBuf>,
        output: Option<impl Into<path::PathBuf>>,
    ) -> Self {
        Entry {
            file: file.into(),
            arguments,
            command: String::default(),
            directory: directory.into(),
            output: output.map(|o| o.into()),
        }
    }

    /// Create an Entry from a shell command string.
    pub fn from_command(
        file: impl Into<path::PathBuf>,
        command: String,
        directory: impl Into<path::PathBuf>,
        output: Option<impl Into<path::PathBuf>>,
    ) -> Self {
        Entry {
            file: file.into(),
            arguments: Vec::default(),
            command,
            directory: directory.into(),
            output: output.map(|o| o.into()),
        }
    }

    /// Semantic validation of the entry. Checking all fields for
    /// valid values and formats.
    pub fn validate(self) -> Result<Self, EntryError> {
        if self.file.to_string_lossy().is_empty() {
            return Err(EntryError::EmptyFileName);
        }
        if self.directory.to_string_lossy().is_empty() {
            return Err(EntryError::EmptyDirectory);
        }
        if self.command.is_empty() && self.arguments.is_empty() {
            return Err(EntryError::CommandOrArgumentsAreMissing);
        }
        if !self.command.is_empty() && !self.arguments.is_empty() {
            return Err(EntryError::CommandOrArgumentsArePresent);
        }
        if !self.command.is_empty() {
            shell_words::split(&self.command)?;
        }
        Ok(self)
    }

    /// Convert entry to a form when only the command field is available.
    ///
    /// The method can fail if the entry is invalid.
    pub fn to_command(self) -> Result<Self, EntryError> {
        let valid = self.validate()?;

        let command = if valid.command.is_empty() {
            shell_words::join(&valid.arguments)
        } else {
            valid.command
        };

        Ok(Entry {
            file: valid.file,
            arguments: Vec::default(),
            command,
            directory: valid.directory,
            output: valid.output,
        })
    }

    /// Convert entry to a form when only the arguments field is available.
    ///
    /// The method can fail if the entry is invalid or command field does
    /// not contain a valid shell escaped string.
    pub fn to_arguments(self) -> Result<Self, EntryError> {
        let valid = self.validate()?;

        let arguments = if valid.arguments.is_empty() {
            shell_words::split(&valid.command)?
        } else {
            valid.arguments
        };

        Ok(Entry {
            file: valid.file,
            arguments,
            command: String::default(),
            directory: valid.directory,
            output: valid.output,
        })
    }

    /// Constructor method for testing purposes.
    #[cfg(test)]
    pub fn from_arguments_str(
        file: &str,
        arguments: Vec<&str>,
        directory: &str,
        output: Option<&str>,
    ) -> Entry {
        Entry::from_arguments(
            path::PathBuf::from(file),
            arguments.into_iter().map(String::from).collect(),
            path::PathBuf::from(directory),
            output.map(path::PathBuf::from),
        )
    }

    /// Constructor method for testing purposes.
    #[cfg(test)]
    pub fn from_command_str(
        file: &str,
        command: &str,
        directory: &str,
        output: Option<&str>,
    ) -> Entry {
        Entry::from_command(
            path::PathBuf::from(file),
            String::from(command),
            path::PathBuf::from(directory),
            output.map(path::PathBuf::from),
        )
    }
}

/// Represents the possible errors that can occur when validating an entry.
#[derive(Debug, Error)]
pub enum EntryError {
    #[error("Entry has an empty file field")]
    EmptyFileName,
    #[error("Entry has an empty directory field")]
    EmptyDirectory,
    #[error("Both command and arguments fields are empty")]
    CommandOrArgumentsAreMissing,
    #[error("Both command and arguments fields are present")]
    CommandOrArgumentsArePresent,
    #[error("Entry has an invalid command field: {0}")]
    InvalidCommand(#[from] shell_words::ParseError),
}

/// Formats `semantic::CompilerCall` instances into `Entry` objects.
pub(super) struct FormattedClangOutputWriter<T: IteratorWriter<Entry>> {
    formatter: formatter::EntryFormatter,
    writer: T,
}

impl<T: IteratorWriter<Entry>> FormattedClangOutputWriter<T> {
    pub(super) fn new(writer: T) -> Self {
        let formatter = formatter::EntryFormatter::new();
        Self { formatter, writer }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<semantic::CompilerCall>
    for FormattedClangOutputWriter<T>
{
    fn write(self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        let entries = semantics.flat_map(|semantic| self.formatter.apply(semantic));
        self.writer.write(entries)
    }
}

/// Handles the logic for appending entries to an existing Clang output file.
///
/// This writer supports reading existing entries from a compilation database file,
/// combining them with new entries, and writing the result back to the file.
/// If the file does not exist and the append option is enabled, it logs a warning
/// and writes only the new entries.
pub(super) struct AppendClangOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    path: Option<path::PathBuf>,
}

impl<T: IteratorWriter<Entry>> AppendClangOutputWriter<T> {
    pub(super) fn new(writer: T, append: bool, file_name: &path::Path) -> Self {
        let path = if file_name.exists() {
            Some(file_name.to_path_buf())
        } else {
            if append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            None
        };
        Self { writer, path }
    }

    /// Reads the compilation database from a file.
    ///
    /// NOTE: The function is intentionally not getting any `&self` reference,
    /// because the logic is not bound to the instance.
    fn read_from_compilation_db(
        source: &path::Path,
    ) -> anyhow::Result<impl Iterator<Item = Entry>> {
        let file = fs::File::open(source)
            .map(io::BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = JsonCompilationDatabase::read_and_ignore(file, |error| {
            log::warn!("Problems to read previous entries: {:?}", error);
        });
        Ok(entries)
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for AppendClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        if let Some(path) = self.path {
            let entries_from_db = Self::read_from_compilation_db(&path)?;
            let final_entries = entries_from_db.chain(entries);
            self.writer.write(final_entries)
        } else {
            self.writer.write(entries)
        }
    }
}

/// Responsible for writing a JSON compilation database file atomically.
///
/// The file is first written to a temporary file and then renamed to the final file name.
/// This ensures that the output file is not left in an inconsistent state in case of errors.
pub(super) struct AtomicClangOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    temp_file_name: path::PathBuf,
    final_file_name: path::PathBuf,
}

impl<T: IteratorWriter<Entry>> AtomicClangOutputWriter<T> {
    pub(super) fn new(
        writer: T,
        temp_file_name: &path::Path,
        final_file_name: &path::Path,
    ) -> Self {
        Self {
            writer,
            temp_file_name: temp_file_name.to_path_buf(),
            final_file_name: final_file_name.to_path_buf(),
        }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for AtomicClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        let temp_file_name = self.temp_file_name.clone();
        let final_file_name = self.final_file_name.clone();

        self.writer.write(entries)?;

        fs::rename(&temp_file_name, &final_file_name).with_context(|| {
            format!(
                "Failed to rename file from '{:?}' to '{:?}'.",
                temp_file_name, final_file_name
            )
        })?;

        Ok(())
    }
}

/// Responsible for writing a JSON compilation database file from the given entries.
///
/// # Features
/// - Filters duplicates based on the provided configuration.
pub(super) struct UniqueOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    filter: filter_duplicates::DuplicateFilter,
}

impl<T: IteratorWriter<Entry>> UniqueOutputWriter<T> {
    pub(super) fn create(writer: T, config: &config::DuplicateFilter) -> anyhow::Result<Self> {
        let filter = filter_duplicates::DuplicateFilter::try_from(config.clone())
            .with_context(|| format!("Failed to create duplicate filter: {:?}", config))?;

        Ok(Self { writer, filter })
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for UniqueOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        let mut filter = self.filter.clone();
        let filtered_entries = entries.filter(move |entry| filter.unique(entry));

        self.writer.write(filtered_entries)
    }
}

/// Responsible for writing a JSON compilation database file from the given entries.
///
/// # Features
/// - Writes the entries to a file.
pub(super) struct ClangOutputWriter {
    output: io::BufWriter<fs::File>,
}

impl ClangOutputWriter {
    pub(super) fn create(file_name: &path::Path) -> anyhow::Result<Self> {
        let output = fs::File::create(file_name)
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        Ok(Self { output })
    }
}

impl IteratorWriter<Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        JsonCompilationDatabase::write(self.output, entries)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self};
    use tempfile::tempdir;

    struct MockWriter;

    impl IteratorWriter<Entry> for MockWriter {
        fn write(self, _: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_atomic_clang_output_writer_success() {
        let dir = tempdir().unwrap();
        let temp_file_path = dir.path().join("temp_file.json");
        let final_file_path = dir.path().join("final_file.json");

        // Create the temp file
        fs::File::create(&temp_file_path).unwrap();

        let sut = AtomicClangOutputWriter::new(MockWriter, &temp_file_path, &final_file_path);
        sut.write(std::iter::empty()).unwrap();

        // Verify the final file exists
        assert!(final_file_path.exists());
        assert!(!temp_file_path.exists());
    }

    #[test]
    fn test_atomic_clang_output_writer_temp_file_missing() {
        let dir = tempdir().unwrap();
        let temp_file_path = dir.path().join("temp_file.json");
        let final_file_path = dir.path().join("final_file.json");

        let sut = AtomicClangOutputWriter::new(MockWriter, &temp_file_path, &final_file_path);
        let result = sut.write(std::iter::empty());

        // Verify the operation fails
        assert!(result.is_err());
        assert!(!final_file_path.exists());
    }

    #[test]
    fn test_atomic_clang_output_writer_final_file_exists() {
        let dir = tempdir().unwrap();
        let temp_file_path = dir.path().join("temp_file.json");
        let final_file_path = dir.path().join("final_file.json");

        // Create the temp file and final file
        fs::File::create(&temp_file_path).unwrap();
        fs::File::create(&final_file_path).unwrap();

        let sut = AtomicClangOutputWriter::new(MockWriter, &temp_file_path, &final_file_path);
        let result = sut.write(std::iter::empty());

        // Verify the operation fails
        assert!(result.is_ok());
        assert!(final_file_path.exists());
        assert!(!temp_file_path.exists());
    }

    #[test]
    fn test_append_clang_output_writer_no_original_file() {
        let dir = tempdir().unwrap();
        let file_to_append = dir.path().join("file_to_append.json");
        let result_file = dir.path().join("result_file.json");

        let entries_to_write = vec![
            Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_file).unwrap();
        let sut = AppendClangOutputWriter::new(writer, false, &file_to_append);
        sut.write(entries_to_write.into_iter()).unwrap();

        // Verify the result file contains the written entries
        assert!(result_file.exists());
        let content = fs::read_to_string(&result_file).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
    }

    #[test]
    fn test_append_clang_output_writer_with_original_file() {
        let dir = tempdir().unwrap();
        let file_to_append = dir.path().join("file_to_append.json");
        let result_file = dir.path().join("result_file.json");

        // Create the original file with some entries
        let original_entries = vec![
            Entry::from_arguments_str("file3.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            Entry::from_arguments_str("file4.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];
        let writer = ClangOutputWriter::create(&file_to_append).unwrap();
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_file).unwrap();
        let sut = AppendClangOutputWriter::new(writer, false, &file_to_append);
        sut.write(new_entries.into_iter()).unwrap();

        // Verify the result file contains both original and new entries
        assert!(result_file.exists());
        let content = fs::read_to_string(&result_file).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
        assert!(content.contains("file3.cpp"));
        assert!(content.contains("file4.cpp"));
    }

    #[test]
    fn test_entry_validate_success_arguments() {
        let entry = Entry::from_arguments_str("main.cpp", vec!["clang", "-c"], "/tmp", None);

        assert!(entry.command.is_empty());
        assert!(!entry.arguments.is_empty());

        assert!(entry.clone().validate().is_ok());
    }

    #[test]
    fn test_entry_validate_success_command() {
        let entry = Entry::from_command_str("main.cpp", "clang -c", "/tmp", None);

        assert!(!entry.command.is_empty());
        assert!(entry.arguments.is_empty());

        assert!(entry.clone().validate().is_ok());
    }

    #[test]
    fn test_entry_validate_errors() {
        let cases = vec![
            (
                Entry::from_arguments_str("", vec!["clang", "-c"], "/tmp", None),
                EntryError::EmptyFileName,
            ),
            (
                Entry::from_arguments_str("main.cpp", vec!["clang", "-c"], "", None),
                EntryError::EmptyDirectory,
            ),
            (
                Entry {
                    file: "main.cpp".into(),
                    arguments: vec![],
                    command: "".to_string(),
                    directory: "/tmp".into(),
                    output: None,
                },
                EntryError::CommandOrArgumentsAreMissing,
            ),
            (
                Entry {
                    file: "main.cpp".into(),
                    arguments: vec!["clang".to_string()],
                    command: "clang".to_string(),
                    directory: "/tmp".into(),
                    output: None,
                },
                EntryError::CommandOrArgumentsArePresent,
            ),
            (
                Entry::from_command_str("main.cpp", "\"unterminated", "/tmp", None),
                EntryError::InvalidCommand(shell_words::ParseError),
            ),
        ];

        for (entry, expected_error) in cases {
            let err = entry.validate().unwrap_err();
            match (err, expected_error) {
                (EntryError::EmptyFileName, EntryError::EmptyFileName)
                | (EntryError::EmptyDirectory, EntryError::EmptyDirectory)
                | (
                    EntryError::CommandOrArgumentsAreMissing,
                    EntryError::CommandOrArgumentsAreMissing,
                )
                | (
                    EntryError::CommandOrArgumentsArePresent,
                    EntryError::CommandOrArgumentsArePresent,
                ) => {}
                (EntryError::InvalidCommand(_), EntryError::InvalidCommand(_)) => {}
                (other, expected) => panic!("Expected {:?}, got {:?}", expected, other),
            }
        }
    }

    #[test]
    fn test_entry_conversions() {
        let entries = vec![
            Entry::from_arguments_str("main.cpp", vec!["clang", "-c", "main.cpp"], "/tmp", None),
            Entry::from_command_str("main.cpp", "clang -c main.cpp", "/tmp", None),
            Entry::from_arguments_str("foo.c", vec!["gcc", "-c", "foo.c"], "/src", Some("foo.o")),
            Entry::from_command_str("bar.c", "gcc -O2 -c bar.c", "/src", Some("bar.o")),
        ];

        for entry in entries {
            // arguments -> command -> arguments
            let to_cmd = entry.clone().to_command().unwrap();
            let to_args = to_cmd.clone().to_arguments().unwrap();
            let to_cmd_again = to_args.clone().to_command().unwrap();
            assert_eq!(to_cmd, to_cmd_again);
            let to_args_again = to_cmd_again.clone().to_arguments().unwrap();
            assert_eq!(to_args, to_args_again);
        }
    }
}

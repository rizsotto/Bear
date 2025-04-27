// SPDX-License-Identifier: GPL-3.0-or-later

//! This crate provides support for reading and writing JSON compilation database files.
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
mod type_de;

use super::formats::{FileFormat, JsonCompilationDatabase};
use super::IteratorWriter;
use crate::{config, semantic};
use anyhow::Context;
use serde::Serialize;
use std::{fs, io, path};

/// Represents an entry of the compilation database.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Entry {
    /// The main translation unit source processed by this compilation step.
    /// This is used by tools as the key into the compilation database.
    /// There can be multiple command objects for the same file, for example if the same
    /// source file is compiled with different configurations.
    pub file: path::PathBuf,
    /// The compile command executed. This must be a valid command to rerun the exact
    /// compilation step for the translation unit in the environment the build system uses.
    /// Shell expansion is not supported.
    pub arguments: Vec<String>,
    /// The working directory of the compilation. All paths specified in the command or
    /// file fields must be either absolute or relative to this directory.
    pub directory: path::PathBuf,
    /// The name of the output created by this compilation step. This field is optional.
    /// It can be used to distinguish different processing modes of the same input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<path::PathBuf>,
}

#[cfg(test)]
pub fn entry(file: &str, arguments: Vec<&str>, directory: &str, output: Option<&str>) -> Entry {
    Entry {
        file: path::PathBuf::from(file),
        arguments: arguments.into_iter().map(String::from).collect(),
        directory: path::PathBuf::from(directory),
        output: output.map(path::PathBuf::from),
    }
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
        let source_copy = source.to_path_buf();

        let file = fs::File::open(source)
            .map(io::BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = JsonCompilationDatabase::read_and_ignore(file, source_copy);
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
            entry("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            entry("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
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
            entry("file3.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            entry("file4.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];
        let writer = ClangOutputWriter::create(&file_to_append).unwrap();
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            entry("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            entry("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
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
}

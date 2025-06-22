// SPDX-License-Identifier: GPL-3.0-or-later

use super::formats::{FileFormat, JsonCompilationDatabase, JsonSemanticDatabase};
use crate::semantic::clang::{DuplicateEntryFilter, Entry};
use crate::semantic::{FormatConfig, Formattable};
use crate::{config, semantic};
use anyhow::Context;
use std::{fs, io, path};

/// The trait represents a writer for iterator type `T`.
///
/// This trait is implemented by types that can consume an iterator of type `T`
/// and write its elements to some output. The writing process may succeed or fail,
/// returning either `()` on success or an error.
pub(super) trait IteratorWriter<T> {
    /// Writes the iterator as a sequence of elements.
    /// It consumes the iterator and returns either a nothing or an error.
    fn write(self, _: impl Iterator<Item = T>) -> anyhow::Result<()>;
}

/// This writer is used to write the semantic analysis results to a file.
///
/// # Note
/// The output format is not stable and may change in future versions.
/// It reflects the internal representation of the semantic analysis types.
pub(super) struct SemanticOutputWriter {
    output: io::BufWriter<fs::File>,
}

impl TryFrom<&path::Path> for SemanticOutputWriter {
    type Error = anyhow::Error;

    fn try_from(file_name: &path::Path) -> Result<Self, Self::Error> {
        let output = fs::File::create(file_name)
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        Ok(Self { output })
    }
}

impl IteratorWriter<semantic::Command> for SemanticOutputWriter {
    fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> anyhow::Result<()> {
        JsonSemanticDatabase::write(self.output, semantics)?;

        Ok(())
    }
}

/// Formats `semantic::CompilerCall` instances into `Entry` objects.
pub(super) struct ConverterClangOutputWriter<T: IteratorWriter<Entry>> {
    format: FormatConfig,
    writer: T,
}

impl<T: IteratorWriter<Entry>> ConverterClangOutputWriter<T> {
    pub(super) fn new(writer: T) -> Self {
        let format = FormatConfig::default();
        Self { format, writer }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<semantic::Command> for ConverterClangOutputWriter<T> {
    fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> anyhow::Result<()> {
        let entries = semantics.flat_map(|semantic| semantic.to_entries(&self.format));
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
    filter: DuplicateEntryFilter,
}

impl<T: IteratorWriter<Entry>> UniqueOutputWriter<T> {
    pub(super) fn create(writer: T, config: &config::DuplicateFilter) -> anyhow::Result<Self> {
        let filter = DuplicateEntryFilter::try_from(config.clone())
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
/// - Formats the entries to the configured shape.
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
}

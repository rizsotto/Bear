// SPDX-License-Identifier: GPL-3.0-or-later

use super::formats::{
    JsonCompilationDatabase, JsonSemanticDatabase, SerializationError, SerializationFormat,
};
use super::{WriterCreationError, WriterError};
use crate::semantic::clang::{DuplicateEntryFilter, Entry};
use crate::{config, semantic};
use std::{fs, io, path};

/// A trait representing a writer for iterator type `T`.
///
/// This trait is implemented by types that can consume an iterator of type `T`
/// and write its elements to some output. The writing process may succeed or fail,
/// returning either `()` on success or an error.
pub(super) trait IteratorWriter<T> {
    /// Writes the iterator as a sequence of elements.
    ///
    /// Consumes the iterator and returns either nothing on success or an error.
    fn write(self, items: impl Iterator<Item = T>) -> Result<(), WriterError>;
}

/// The type represents a writer for semantic analysis results to a file.
///
/// # Note
/// The output format is not stable and may change in future versions.
/// It reflects the internal representation of the semantic analysis types.
pub(super) struct SemanticOutputWriter {
    output: io::BufWriter<fs::File>,
    path: path::PathBuf,
}

impl TryFrom<&path::Path> for SemanticOutputWriter {
    type Error = WriterCreationError;

    fn try_from(output_path: &path::Path) -> Result<Self, Self::Error> {
        let output = fs::File::create(output_path)
            .map(io::BufWriter::new)
            .map_err(|err| WriterCreationError::Io(output_path.to_path_buf(), err))?;

        Ok(Self {
            output,
            path: output_path.to_path_buf(),
        })
    }
}

impl IteratorWriter<semantic::Command> for SemanticOutputWriter {
    fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        JsonSemanticDatabase::write(self.output, semantics)
            .map_err(|err| WriterError::Io(self.path, err))
    }
}

/// The type represents a converter that formats `semantic::Command` instances into `Entry` objects.
pub(super) struct ConverterClangOutputWriter<T: IteratorWriter<Entry>> {
    format: config::EntryFormat,
    writer: T,
}

impl<T: IteratorWriter<Entry>> ConverterClangOutputWriter<T> {
    pub(super) fn new(writer: T, format: &config::EntryFormat) -> Self {
        Self {
            format: format.clone(),
            writer,
        }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<semantic::Command> for ConverterClangOutputWriter<T> {
    fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        let entries = semantics.flat_map(|semantic| semantic.to_entries(&self.format));
        self.writer.write(entries)
    }
}

/// The type represents a writer that handles appending entries to an existing Clang output file.
///
/// This writer supports reading existing entries from a compilation database file,
/// combining them with new entries, and writing the result back to the file.
/// If the file does not exist and the append option is enabled, it logs a warning
/// and writes only the new entries.
///
/// # Note
/// Reading errors will be ignored, and a warning will be logged.
pub(super) struct AppendClangOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    path: Option<path::PathBuf>,
}

impl<T: IteratorWriter<Entry>> AppendClangOutputWriter<T> {
    pub(super) fn new(writer: T, input_path: &path::Path, append: bool) -> Self {
        let path = if input_path.exists() {
            Some(input_path.to_path_buf())
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
    ) -> Result<impl Iterator<Item = Entry>, SerializationError> {
        let file = fs::File::open(source).map(io::BufReader::new)?;

        let entries = JsonCompilationDatabase::read_and_ignore(file, |error| {
            log::warn!("Problems to read previous entries: {error:?}");
        });
        Ok(entries)
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for AppendClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        if let Some(path) = &self.path {
            let entries_from_db = Self::read_from_compilation_db(path)
                .map_err(|err| WriterError::Io(path.clone(), err))?;
            let final_entries = entries_from_db.chain(entries);
            self.writer.write(final_entries)
        } else {
            self.writer.write(entries)
        }
    }
}

/// The type represents a writer that writes JSON compilation database files atomically.
///
/// The file is first written to a temporary file and then renamed to the final file name.
/// This ensures that the output file is not left in an inconsistent state in case of errors.
pub(super) struct AtomicClangOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    temp_path: path::PathBuf,
    final_path: path::PathBuf,
}

impl<T: IteratorWriter<Entry>> AtomicClangOutputWriter<T> {
    pub(super) fn new(writer: T, temp_path: &path::Path, final_path: &path::Path) -> Self {
        Self {
            writer,
            temp_path: temp_path.to_path_buf(),
            final_path: final_path.to_path_buf(),
        }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for AtomicClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        self.writer.write(entries)?;

        fs::rename(&self.temp_path, &self.final_path)
            .map_err(|err| WriterError::Io(self.final_path, SerializationError::Io(err)))?;

        Ok(())
    }
}

/// The type represents a writer that writes JSON compilation database files from given entries.
///
/// # Features
/// - Filters duplicates based on the provided configuration.
pub(super) struct UniqueOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    filter: DuplicateEntryFilter,
}

impl<T: IteratorWriter<Entry>> UniqueOutputWriter<T> {
    pub(super) fn create(
        writer: T,
        config: &config::DuplicateFilter,
    ) -> Result<Self, WriterCreationError> {
        let filter = DuplicateEntryFilter::try_from(config.clone())
            .map_err(|err| WriterCreationError::Configuration(err.to_string()))?;

        Ok(Self { writer, filter })
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for UniqueOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        let mut filter = self.filter.clone();
        let filtered_entries = entries.filter(move |entry| filter.unique(entry));

        self.writer.write(filtered_entries)
    }
}

/// The type represents a writer that writes JSON compilation database files from given entries.
///
/// # Features
/// - Writes the entries to a file.
/// - Formats the entries to the configured shape.
pub(super) struct ClangOutputWriter {
    output: io::BufWriter<fs::File>,
    path: path::PathBuf,
}

impl ClangOutputWriter {
    pub(super) fn create(path: &path::Path) -> Result<Self, WriterCreationError> {
        let output = fs::File::create(path)
            .map(io::BufWriter::new)
            .map_err(|err| WriterCreationError::Io(path.to_path_buf(), err))?;

        Ok(Self {
            output,
            path: path.to_path_buf(),
        })
    }
}

impl IteratorWriter<Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        JsonCompilationDatabase::write(self.output, entries)
            .map_err(|err| WriterError::Io(self.path, err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self};
    use tempfile::tempdir;

    struct MockWriter;

    impl IteratorWriter<Entry> for MockWriter {
        fn write(self, _: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
            Ok(())
        }
    }

    #[test]
    fn test_atomic_clang_output_writer_success() {
        let dir = tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");
        let final_path = dir.path().join("final_file.json");

        // Create the temp file
        fs::File::create(&temp_path).unwrap();

        let sut = AtomicClangOutputWriter::new(MockWriter, &temp_path, &final_path);
        sut.write(std::iter::empty()).unwrap();

        // Verify the final file exists
        assert!(final_path.exists());
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_atomic_clang_output_writer_temp_file_missing() {
        let dir = tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");
        let final_path = dir.path().join("final_file.json");

        let sut = AtomicClangOutputWriter::new(MockWriter, &temp_path, &final_path);
        let result = sut.write(std::iter::empty());

        // Verify the operation fails
        assert!(result.is_err());
        assert!(!final_path.exists());
    }

    #[test]
    fn test_atomic_clang_output_writer_final_file_exists() {
        let dir = tempdir().unwrap();
        let temp_path = dir.path().join("temp_file.json");
        let final_path = dir.path().join("final_file.json");

        // Create the temp file and final file
        fs::File::create(&temp_path).unwrap();
        fs::File::create(&final_path).unwrap();

        let sut = AtomicClangOutputWriter::new(MockWriter, &temp_path, &final_path);
        let result = sut.write(std::iter::empty());

        // Verify the operation fails
        assert!(result.is_ok());
        assert!(final_path.exists());
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_append_clang_output_writer_no_original_file() {
        let dir = tempdir().unwrap();
        let input_path = dir.path().join("file_to_append.json");
        let result_path = dir.path().join("result_file.json");

        let entries_to_write = vec![
            Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_path).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, false);
        sut.write(entries_to_write.into_iter()).unwrap();

        // Verify the result file contains the written entries
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
    }

    #[test]
    fn test_append_clang_output_writer_with_original_file() {
        let dir = tempdir().unwrap();
        let input_path = dir.path().join("file_to_append.json");
        let result_path = dir.path().join("result_file.json");

        // Create the original file with some entries
        let original_entries = vec![
            Entry::from_arguments_str("file3.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            Entry::from_arguments_str("file4.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];
        let writer = ClangOutputWriter::create(&input_path).unwrap();
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_path).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, false);
        sut.write(new_entries.into_iter()).unwrap();

        // Verify the result file contains both original and new entries
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
        assert!(content.contains("file3.cpp"));
        assert!(content.contains("file4.cpp"));
    }
}

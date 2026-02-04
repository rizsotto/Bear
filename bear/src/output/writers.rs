// SPDX-License-Identifier: GPL-3.0-or-later

use super::clang;
use super::formats::{JsonCompilationDatabase, SerializationError, SerializationFormat};
use super::statistics::OutputStatistics;
use super::{WriterCreationError, WriterError};
use crate::{config, semantic};
use std::sync::Arc;
use std::sync::atomic::Ordering;
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

/// The type represents a converter that formats `semantic::Command` instances into `Entry` objects.
pub(super) struct ConverterClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    converter: clang::CommandConverter,
    writer: T,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> ConverterClangOutputWriter<T> {
    pub(super) fn new(writer: T, format: &config::Format, stats: Arc<OutputStatistics>) -> Self {
        Self { converter: clang::CommandConverter::new(format.clone()), writer, stats }
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<semantic::Command> for ConverterClangOutputWriter<T> {
    fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        let stats = Arc::clone(&self.stats);
        let stats_for_entries = Arc::clone(&self.stats);

        // Count semantic commands as they flow in
        let counted_semantics = semantics.inspect(move |_| {
            stats.semantic_commands_received.fetch_add(1, Ordering::Relaxed);
        });

        // Convert and count entries produced
        let entries = counted_semantics.flat_map(|semantic| self.converter.to_entries(&semantic));
        let counted_entries = entries.inspect(move |_| {
            stats_for_entries.compilation_entries_produced.fetch_add(1, Ordering::Relaxed);
        });

        self.writer.write(counted_entries)
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
pub(super) struct AppendClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    path: Option<path::PathBuf>,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> AppendClangOutputWriter<T> {
    pub(super) fn new(
        writer: T,
        input_path: &path::Path,
        append: bool,
        stats: Arc<OutputStatistics>,
    ) -> Self {
        let path = if input_path.exists() && append {
            Some(input_path.to_path_buf())
        } else {
            if append && !input_path.exists() {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            None
        };
        Self { writer, path, stats }
    }

    /// Reads the compilation database from a file.
    ///
    /// NOTE: The function is intentionally not getting any `&self` reference,
    /// because the logic is not bound to the instance.
    fn read_from_compilation_db(
        source: &path::Path,
    ) -> Result<impl Iterator<Item = clang::Entry>, SerializationError> {
        let file = fs::File::open(source).map(io::BufReader::new)?;

        let entries = JsonCompilationDatabase::read_and_ignore(file, |error| {
            log::warn!("Problems to read previous entries: {error:?}");
        });
        Ok(entries)
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for AppendClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
        if let Some(path) = &self.path {
            let stats = Arc::clone(&self.stats);

            let entries_from_db =
                Self::read_from_compilation_db(path).map_err(|err| WriterError::Io(path.clone(), err))?;

            // Count entries read from existing database
            let counted_existing = entries_from_db.inspect(move |_| {
                stats.entries_read_from_existing.fetch_add(1, Ordering::Relaxed);
            });

            let final_entries = counted_existing.chain(entries);
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
pub(super) struct AtomicClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    temp_path: path::PathBuf,
    final_path: path::PathBuf,
}

impl<T: IteratorWriter<clang::Entry>> AtomicClangOutputWriter<T> {
    pub(super) fn new(writer: T, temp_path: &path::Path, final_path: &path::Path) -> Self {
        Self { writer, temp_path: temp_path.to_path_buf(), final_path: final_path.to_path_buf() }
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for AtomicClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
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
pub(super) struct UniqueOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    filter: clang::DuplicateEntryFilter,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> UniqueOutputWriter<T> {
    pub(super) fn create(
        writer: T,
        config: config::DuplicateFilter,
        stats: Arc<OutputStatistics>,
    ) -> Result<Self, WriterCreationError> {
        let filter = clang::DuplicateEntryFilter::try_from(config)
            .map_err(|err| WriterCreationError::Configuration(err.to_string()))?;

        Ok(Self { writer, filter, stats })
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for UniqueOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
        let mut filter = self.filter;
        let stats = Arc::clone(&self.stats);

        let filtered_entries = entries.filter(move |entry| {
            let is_unique = filter.unique(entry);
            if !is_unique {
                stats.duplicates_detected.fetch_add(1, Ordering::Relaxed);
            }
            is_unique
        });

        self.writer.write(filtered_entries)
    }
}

/// The type represents a writer that filters compilation database entries based on source file paths.
///
/// # Features
/// - Filters entries based on directory-based rules with order-based evaluation semantics.
/// - Uses the configured source filter to include/exclude files.
pub(super) struct SourceFilterOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    filter: clang::SourceEntryFilter,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> SourceFilterOutputWriter<T> {
    pub(super) fn create(
        writer: T,
        config: config::SourceFilter,
        stats: Arc<OutputStatistics>,
    ) -> Result<Self, WriterCreationError> {
        let filter = clang::SourceEntryFilter::try_from(config)
            .map_err(|err| WriterCreationError::Configuration(err.to_string()))?;

        Ok(Self { writer, filter, stats })
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for SourceFilterOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
        let filter = self.filter;
        let stats = Arc::clone(&self.stats);

        let filtered_entries = entries.filter(move |entry| {
            let included = filter.should_include(entry);
            if !included {
                stats.entries_filtered_by_source.fetch_add(1, Ordering::Relaxed);
            }
            included
        });

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
    stats: Arc<OutputStatistics>,
}

impl ClangOutputWriter {
    pub(super) fn create(
        path: &path::Path,
        stats: Arc<OutputStatistics>,
    ) -> Result<Self, WriterCreationError> {
        let output = fs::File::create(path)
            .map(io::BufWriter::new)
            .map_err(|err| WriterCreationError::Io(path.to_path_buf(), err))?;

        Ok(Self { output, path: path.to_path_buf(), stats })
    }
}

impl IteratorWriter<clang::Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
        let stats = Arc::clone(&self.stats);

        // Count entries as they are written
        let counted_entries = entries.inspect(move |_| {
            stats.entries_written.fetch_add(1, Ordering::Relaxed);
        });

        JsonCompilationDatabase::write(self.output, counted_entries)
            .map_err(|err| WriterError::Io(self.path, err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryAction, DirectoryRule, SourceFilter};
    use std::fs::{self};
    use std::path::PathBuf;
    use tempfile::tempdir;

    struct MockWriter;

    impl IteratorWriter<clang::Entry> for MockWriter {
        fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
            // Consume the iterator to trigger upstream counting
            entries.for_each(drop);
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
        let stats = OutputStatistics::new();

        let entries_to_write = vec![
            clang::Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_path, Arc::clone(&stats)).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, false, Arc::clone(&stats));
        sut.write(entries_to_write.into_iter()).unwrap();

        // Verify the result file contains the written entries
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
    }

    #[test]
    fn test_source_filter_output_writer_includes_matching_entries() {
        let stats = OutputStatistics::new();
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };

        let sut = SourceFilterOutputWriter::create(MockWriter, config, Arc::clone(&stats)).unwrap();

        let entries = vec![
            clang::Entry::from_arguments_str("src/main.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("/usr/include/stdio.h", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("lib/utils.c", vec!["gcc", "-c"], "/project", None),
        ];

        // This would normally write, but MockWriter doesn't actually write
        // The test verifies the writer can be created and configured properly
        assert!(sut.write(entries.into_iter()).is_ok());
    }

    #[test]
    fn test_source_filter_output_writer_empty_config() {
        let stats = OutputStatistics::new();
        let config = SourceFilter::default();

        let sut = SourceFilterOutputWriter::create(MockWriter, config, Arc::clone(&stats)).unwrap();

        let entries =
            vec![clang::Entry::from_arguments_str("any/file.c", vec!["gcc", "-c"], "/project", None)];

        assert!(sut.write(entries.into_iter()).is_ok());
    }

    #[test]
    fn test_source_filter_output_writer_complex_rules() {
        let stats = OutputStatistics::new();
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("/home/project"), action: DirectoryAction::Include },
                DirectoryRule {
                    path: PathBuf::from("/home/project/build"),
                    action: DirectoryAction::Exclude,
                },
            ],
        };

        let sut = SourceFilterOutputWriter::create(MockWriter, config, Arc::clone(&stats)).unwrap();

        let entries = vec![
            clang::Entry::from_arguments_str("./src/main.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("build/main.o", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("build/config/defs.h", vec!["gcc", "-c"], "/project", None),
        ];

        assert!(sut.write(entries.into_iter()).is_ok());
    }

    #[test]
    fn test_source_filter_integration_with_writer_pipeline() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("compile_commands.json");
        let stats = OutputStatistics::new();

        // Create a source filter configuration
        let source_config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };

        // Create a duplicate filter configuration
        let duplicate_config = config::DuplicateFilter {
            match_on: vec![config::OutputFields::File, config::OutputFields::Directory],
        };

        // Build the writer pipeline: base -> unique -> source_filter
        let base_writer = ClangOutputWriter::create(&output_path, Arc::clone(&stats)).unwrap();
        let unique_writer =
            UniqueOutputWriter::create(base_writer, duplicate_config, Arc::clone(&stats)).unwrap();
        let source_filter_writer =
            SourceFilterOutputWriter::create(unique_writer, source_config, Arc::clone(&stats)).unwrap();

        // Test entries: some should be filtered, some should pass through
        let entries = vec![
            clang::Entry::from_arguments_str("src/main.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("/usr/include/stdio.h", vec!["gcc", "-c"], "/project", None), // should be excluded
            clang::Entry::from_arguments_str("lib/utils.c", vec!["gcc", "-c"], "/project", None), // should be included (no match)
            clang::Entry::from_arguments_str("src/helper.c", vec!["gcc", "-c"], "/project", None),
        ];

        // Write through the pipeline
        assert!(source_filter_writer.write(entries.into_iter()).is_ok());

        // Verify the file was created
        assert!(output_path.exists());

        // Read and verify the content (basic check that something was written)
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("src/main.c"));
        assert!(content.contains("lib/utils.c"));
        assert!(content.contains("src/helper.c"));
        assert!(!content.contains("/usr/include/stdio.h")); // This should be filtered out
    }

    #[test]
    fn test_append_clang_output_writer_with_original_file() {
        let dir = tempdir().unwrap();
        let input_path = dir.path().join("file_to_append.json");
        let result_path = dir.path().join("result_file.json");
        let stats = OutputStatistics::new();

        // Create the original file with some entries
        let original_entries = vec![
            clang::Entry::from_arguments_str("file3.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("file4.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];
        let writer = ClangOutputWriter::create(&input_path, Arc::clone(&stats)).unwrap();
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            clang::Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_path, Arc::clone(&stats)).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, true, Arc::clone(&stats));
        sut.write(new_entries.into_iter()).unwrap();

        // Verify the result file contains both original and new entries
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
        assert!(content.contains("file3.cpp"));
        assert!(content.contains("file4.cpp"));
    }

    #[test]
    fn test_append_clang_output_writer_overwrite_existing_file() {
        let dir = tempdir().unwrap();
        let input_path = dir.path().join("file_to_overwrite.json");
        let result_path = dir.path().join("result_file.json");
        let stats = OutputStatistics::new();

        // Create the original file with some entries
        let original_entries = vec![
            clang::Entry::from_arguments_str("old_file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("old_file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];
        let writer = ClangOutputWriter::create(&input_path, Arc::clone(&stats)).unwrap();
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            clang::Entry::from_arguments_str("new_file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("new_file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::create(&result_path, Arc::clone(&stats)).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, false, Arc::clone(&stats));
        sut.write(new_entries.into_iter()).unwrap();

        // Verify the result file contains only new entries (no original entries)
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("new_file1.cpp"));
        assert!(content.contains("new_file2.cpp"));
        assert!(!content.contains("old_file1.cpp"));
        assert!(!content.contains("old_file2.cpp"));
    }
}

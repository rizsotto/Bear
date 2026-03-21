// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang;
use crate::output::clang::serialization::JsonCompilationDatabase;
use crate::output::formats::{SerializationError, SerializationFormat};
use crate::output::statistics::OutputStatistics;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{fs, io, path};

/// The type represents a writer that handles appending entries to an existing Clang output file.
///
/// This writer supports reading existing entries from a compilation database file,
/// combining them with new entries, and writing the result back to the file.
/// If the file does not exist and the append option is enabled, it logs a warning
/// and writes only the new entries.
///
/// # Note
/// Reading errors will be ignored, and a warning will be logged.
pub(crate) struct AppendClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    path: Option<path::PathBuf>,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> AppendClangOutputWriter<T> {
    pub(crate) fn new(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::writers::file::ClangOutputWriter;

    #[test]
    fn test_append_clang_output_writer_no_original_file() {
        let dir = tempfile::tempdir().unwrap();
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
    fn test_append_clang_output_writer_with_original_file() {
        let dir = tempfile::tempdir().unwrap();
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
    fn test_append_with_corrupted_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("corrupted.json");
        let result_path = dir.path().join("result_file.json");
        let stats = OutputStatistics::new();

        // Write invalid JSON to the input file
        fs::write(&input_path, "this is not valid json at all!!!").unwrap();

        let new_entries =
            vec![clang::Entry::from_arguments_str("new_file.cpp", vec!["clang", "-c"], "/path/to/dir", None)];

        let writer = ClangOutputWriter::create(&result_path, Arc::clone(&stats)).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, true, Arc::clone(&stats));

        // Should fail because read_from_compilation_db returns an error for non-JSON
        // (the file opens fine, but deserialization fails during iteration —
        //  however read_and_ignore swallows errors, so the iterator just yields nothing)
        // Actually: read_from_compilation_db calls read_and_ignore which filters errors.
        // So it should succeed with zero entries from existing DB.
        sut.write(new_entries.into_iter()).unwrap();

        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("new_file.cpp"));
        assert_eq!(stats.entries_read_from_existing.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[test]
    fn test_append_with_partially_valid_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("partial.json");
        let result_path = dir.path().join("result_file.json");
        let stats = OutputStatistics::new();

        // Write a JSON array with one valid and one invalid entry
        // The valid entry has both command and arguments (which is invalid per validation)
        let partial_content = r#"[
            {"directory": "/path/to/dir", "file": "valid.cpp", "arguments": ["clang", "-c"]},
            {"directory": "", "file": "invalid.cpp", "arguments": ["clang", "-c"]}
        ]"#;
        fs::write(&input_path, partial_content).unwrap();

        let new_entries =
            vec![clang::Entry::from_arguments_str("new_file.cpp", vec!["clang", "-c"], "/path/to/dir", None)];

        let writer = ClangOutputWriter::create(&result_path, Arc::clone(&stats)).unwrap();
        let sut = AppendClangOutputWriter::new(writer, &input_path, true, Arc::clone(&stats));
        sut.write(new_entries.into_iter()).unwrap();

        let content = fs::read_to_string(&result_path).unwrap();
        // Valid existing entry should be included
        assert!(content.contains("valid.cpp"));
        // New entry should be included
        assert!(content.contains("new_file.cpp"));
        // Invalid entry should be skipped (read_and_ignore filters it out)
        assert!(!content.contains("invalid.cpp"));
        // Only the valid existing entry was counted
        assert_eq!(stats.entries_read_from_existing.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn test_append_clang_output_writer_overwrite_existing_file() {
        let dir = tempfile::tempdir().unwrap();
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

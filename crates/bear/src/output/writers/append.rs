// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterCreationError;
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
/// The existing file is opened once when the writer is created, so a file
/// that cannot be opened fails before the build runs. It is read after the
/// build, as it is then. Invalid entries in it are skipped with a warning.
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
    ) -> Result<Self, WriterCreationError> {
        let path = if input_path.exists() && append {
            fs::File::open(input_path)
                .map_err(|err| WriterCreationError::Read(input_path.to_path_buf(), err))?;
            Some(input_path.to_path_buf())
        } else {
            if append && !input_path.exists() {
                log::warn!(
                    "cannot read previous entries from {}: the file does not exist",
                    input_path.display()
                );
            }
            None
        };
        Ok(Self { writer, path, stats })
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for AppendClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), SerializationError> {
        if let Some(path) = self.path {
            let stats = Arc::clone(&self.stats);

            // Opened before the build already; a failure now means the file
            // changed during the build.
            let file = fs::File::open(&path)?;

            let entries_from_db =
                JsonCompilationDatabase::read_and_ignore(io::BufReader::new(file), move |error| {
                    log::warn!("cannot read previous entries from {}: {error}", path.display());
                });

            // Count entries read from existing database
            let counted_existing = entries_from_db.inspect(move |_| {
                stats.entries_read_from_existing.fetch_add(1, Ordering::Relaxed);
            });

            let final_entries = entries.chain(counted_existing);
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

        let writer = ClangOutputWriter::new(fs::File::create(&result_path).unwrap(), Arc::clone(&stats));
        let sut = AppendClangOutputWriter::new(writer, &input_path, false, Arc::clone(&stats)).unwrap();
        sut.write(entries_to_write.into_iter()).unwrap();

        // Verify the result file contains the written entries
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
    }

    // Requirements: output-append
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
        let writer = ClangOutputWriter::new(fs::File::create(&input_path).unwrap(), Arc::clone(&stats));
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            clang::Entry::from_arguments_str("file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::new(fs::File::create(&result_path).unwrap(), Arc::clone(&stats));
        let sut = AppendClangOutputWriter::new(writer, &input_path, true, Arc::clone(&stats)).unwrap();
        sut.write(new_entries.into_iter()).unwrap();

        // Verify the result file contains both original and new entries
        assert!(result_path.exists());
        let content = fs::read_to_string(&result_path).unwrap();
        assert!(content.contains("file1.cpp"));
        assert!(content.contains("file2.cpp"));
        assert!(content.contains("file3.cpp"));
        assert!(content.contains("file4.cpp"));

        // New entries must be emitted before the existing ones.
        let pos_new1 = content.find("file1.cpp").unwrap();
        let pos_new2 = content.find("file2.cpp").unwrap();
        let pos_existing3 = content.find("file3.cpp").unwrap();
        let pos_existing4 = content.find("file4.cpp").unwrap();
        assert!(
            pos_new1 < pos_existing3 && pos_new2 < pos_existing3,
            "new entries must come before existing entries in the merged output"
        );
        assert!(
            pos_new1 < pos_existing4 && pos_new2 < pos_existing4,
            "new entries must come before existing entries in the merged output"
        );
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

        let writer = ClangOutputWriter::new(fs::File::create(&result_path).unwrap(), Arc::clone(&stats));
        let sut = AppendClangOutputWriter::new(writer, &input_path, true, Arc::clone(&stats)).unwrap();

        // The invalid content is skipped with a warning, so only the new entry is written.
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

        let writer = ClangOutputWriter::new(fs::File::create(&result_path).unwrap(), Arc::clone(&stats));
        let sut = AppendClangOutputWriter::new(writer, &input_path, true, Arc::clone(&stats)).unwrap();
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
        let writer = ClangOutputWriter::new(fs::File::create(&input_path).unwrap(), Arc::clone(&stats));
        writer.write(original_entries.into_iter()).unwrap();

        let new_entries = vec![
            clang::Entry::from_arguments_str("new_file1.cpp", vec!["clang", "-c"], "/path/to/dir", None),
            clang::Entry::from_arguments_str("new_file2.cpp", vec!["clang", "-c"], "/path/to/dir", None),
        ];

        let writer = ClangOutputWriter::new(fs::File::create(&result_path).unwrap(), Arc::clone(&stats));
        let sut = AppendClangOutputWriter::new(writer, &input_path, false, Arc::clone(&stats)).unwrap();
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

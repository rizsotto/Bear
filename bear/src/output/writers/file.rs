// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::clang;
use crate::output::clang::serialization::JsonCompilationDatabase;
use crate::output::formats::SerializationFormat;
use crate::output::statistics::OutputStatistics;
use crate::output::{WriterCreationError, WriterError};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{fs, io, path};

/// The type represents a writer that writes JSON compilation database files from given entries.
///
/// # Features
/// - Writes the entries to a file.
/// - Formats the entries to the configured shape.
pub(crate) struct ClangOutputWriter {
    output: io::BufWriter<fs::File>,
    path: path::PathBuf,
    stats: Arc<OutputStatistics>,
}

impl ClangOutputWriter {
    pub(crate) fn create(
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
    use crate::output::statistics::OutputStatistics;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_create_and_write_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.json");
        let stats = OutputStatistics::new();

        let entries = vec![
            clang::Entry::from_arguments_str("file1.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("file2.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("file3.c", vec!["gcc", "-c"], "/project", Some("file3.o")),
        ];

        let writer = ClangOutputWriter::create(&path, Arc::clone(&stats)).unwrap();
        writer.write(entries.into_iter()).unwrap();

        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 3);

        // Read back and verify
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("file1.c"));
        assert!(content.contains("file2.c"));
        assert!(content.contains("file3.c"));
        assert!(content.contains("file3.o"));
    }

    #[test]
    fn test_create_failure_invalid_path() {
        let stats = OutputStatistics::new();
        let result = ClangOutputWriter::create(
            path::Path::new("/nonexistent/directory/output.json"),
            Arc::clone(&stats),
        );

        assert!(result.is_err());
        match result {
            Err(WriterCreationError::Io(p, _)) => {
                assert_eq!(p, path::PathBuf::from("/nonexistent/directory/output.json"));
            }
            _ => panic!("Expected WriterCreationError::Io"),
        }
    }

    #[test]
    fn test_entries_written_counter_matches_actual() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.json");
        let stats = OutputStatistics::new();

        let writer = ClangOutputWriter::create(&path, Arc::clone(&stats)).unwrap();
        writer.write(std::iter::empty()).unwrap();

        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 0);

        let content = fs::read_to_string(&path).unwrap();
        // Should be an empty JSON array
        assert!(content.contains("[]") || content.trim() == "[\n]" || content.trim().starts_with('['));
    }
}

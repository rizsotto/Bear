// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::SerializationError;
use crate::output::clang;
use crate::output::clang::serialization::JsonCompilationDatabase;
use crate::output::formats::SerializationFormat;
use crate::output::statistics::OutputStatistics;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{fs, io};

/// The type represents a writer that writes JSON compilation database files from given entries.
///
/// # Features
/// - Writes the entries to a file.
/// - Formats the entries to the configured shape.
pub(crate) struct ClangOutputWriter {
    output: io::BufWriter<fs::File>,
    stats: Arc<OutputStatistics>,
}

impl ClangOutputWriter {
    pub(crate) fn new(file: fs::File, stats: Arc<OutputStatistics>) -> Self {
        Self { output: io::BufWriter::new(file), stats }
    }
}

impl IteratorWriter<clang::Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), SerializationError> {
        write_entries(self.output, entries, self.stats)
    }
}

/// Serializes `entries` as a JSON compilation database to `output`, counting
/// each entry via `stats`. The caller adds the path to an error.
///
/// Shared by the file-backed [`ClangOutputWriter`] and the stdout-backed
/// `ClangStdoutOutputWriter` (see `writers/stdout.rs`) so both stream through
/// the identical serialization instead of duplicating it.
pub(super) fn write_entries(
    output: impl io::Write,
    entries: impl Iterator<Item = clang::Entry>,
    stats: Arc<OutputStatistics>,
) -> Result<(), SerializationError> {
    // Count entries as they are written
    let counted_entries = entries.inspect(move |_| {
        stats.entries_written.fetch_add(1, Ordering::Relaxed);
    });

    JsonCompilationDatabase::write(output, counted_entries)
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

        let writer = ClangOutputWriter::new(fs::File::create(&path).unwrap(), Arc::clone(&stats));
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
    fn test_entries_written_counter_matches_actual() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.json");
        let stats = OutputStatistics::new();

        let writer = ClangOutputWriter::new(fs::File::create(&path).unwrap(), Arc::clone(&stats));
        writer.write(std::iter::empty()).unwrap();

        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 0);

        let content = fs::read_to_string(&path).unwrap();
        // Should be an empty JSON array
        assert!(content.contains("[]") || content.trim() == "[\n]" || content.trim().starts_with('['));
    }
}

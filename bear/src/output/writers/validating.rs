// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang::Entry;
use crate::output::statistics::OutputStatistics;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Drops entries that fail `Entry::validate`, logs each drop at `WARN` level,
/// and increments `OutputStatistics::entries_dropped_invalid`.
///
/// Placed last in the pipeline so earlier filter stages (duplicate, source)
/// never see an entry that will be dropped. Downstream writers
/// (`ClangOutputWriter`) may therefore assume every entry they receive is
/// valid per `Entry::validate`.
pub(crate) struct ValidatingOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<Entry>> ValidatingOutputWriter<T> {
    pub(crate) fn new(writer: T, stats: Arc<OutputStatistics>) -> Self {
        Self { writer, stats }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for ValidatingOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        let stats = Arc::clone(&self.stats);

        let valid_entries = entries.filter_map(move |entry| match entry.validate() {
            Ok(()) => Some(entry),
            Err(reason) => {
                stats.entries_dropped_invalid.fetch_add(1, Ordering::Relaxed);
                log::warn!(
                    "Dropping invalid compilation database entry for file {:?} in directory {:?}: {}",
                    entry.file,
                    entry.directory,
                    reason
                );
                None
            }
        });

        self.writer.write(valid_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::writers::fixtures::CollectingWriter;

    fn valid_entry(file: &str) -> Entry {
        Entry::from_arguments_str(file, vec!["gcc", "-c"], "/project", None)
    }

    fn entry_with_empty_directory() -> Entry {
        Entry::from_arguments_str("main.c", vec!["gcc", "-c"], "", None)
    }

    fn entry_with_empty_file() -> Entry {
        Entry::from_arguments_str("", vec!["gcc", "-c"], "/project", None)
    }

    #[test]
    fn test_validating_writer_passes_valid_entries_unchanged() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = ValidatingOutputWriter::new(writer, Arc::clone(&stats));

        let entries = vec![valid_entry("a.c"), valid_entry("b.c"), valid_entry("c.c")];
        sut.write(entries.into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(stats.entries_dropped_invalid.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_validating_writer_drops_invalid_entries_and_keeps_valid_ones() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = ValidatingOutputWriter::new(writer, Arc::clone(&stats));

        let entries = vec![
            valid_entry("a.c"),
            entry_with_empty_directory(),
            valid_entry("b.c"),
            entry_with_empty_file(),
            valid_entry("c.c"),
        ];
        sut.write(entries.into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].file, std::path::PathBuf::from("a.c"));
        assert_eq!(out[1].file, std::path::PathBuf::from("b.c"));
        assert_eq!(out[2].file, std::path::PathBuf::from("c.c"));
        assert_eq!(stats.entries_dropped_invalid.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_validating_writer_drops_every_entry() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = ValidatingOutputWriter::new(writer, Arc::clone(&stats));

        let entries = vec![entry_with_empty_directory(), entry_with_empty_file()];
        sut.write(entries.into_iter()).unwrap();

        let out = collected.lock().unwrap();
        assert!(out.is_empty());
        assert_eq!(stats.entries_dropped_invalid.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_validating_writer_empty_input() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let sut = ValidatingOutputWriter::new(writer, Arc::clone(&stats));

        sut.write(std::iter::empty()).unwrap();

        let out = collected.lock().unwrap();
        assert!(out.is_empty());
        assert_eq!(stats.entries_dropped_invalid.load(Ordering::Relaxed), 0);
    }
}

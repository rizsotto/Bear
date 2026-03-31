// SPDX-License-Identifier: GPL-3.0-or-later

//! Entry filtering for the output pipeline.
//!
//! This module provides the [`EntryFilter`] trait and a generic
//! [`FilteredOutputWriter`] that removes entries from the stream based on
//! pluggable filter implementations (source-path rules, duplicate detection).

pub(crate) mod source;
pub(crate) mod unique;

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang::Entry;
use crate::output::statistics::OutputStatistics;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) use source::SourceEntryFilter;
pub(crate) use unique::DuplicateEntryFilter;

/// A trait for filtering compilation database entries.
///
/// Implementations decide whether an entry should be kept or removed from the
/// output pipeline. Uses `&mut self` to support stateful filters (e.g. dedup).
pub(crate) trait EntryFilter {
    fn accept(&mut self, entry: &Entry) -> bool;
}

/// A generic pipeline writer that filters entries using an [`EntryFilter`]
/// and delegates to an inner writer.
pub(crate) struct FilteredOutputWriter<T: IteratorWriter<Entry>, F: EntryFilter> {
    writer: T,
    filter: F,
    stats: Arc<OutputStatistics>,
    rejected_counter: fn(&OutputStatistics) -> &AtomicUsize,
}

impl<T: IteratorWriter<Entry>, F: EntryFilter> FilteredOutputWriter<T, F> {
    pub(crate) fn new(
        writer: T,
        filter: F,
        stats: Arc<OutputStatistics>,
        rejected_counter: fn(&OutputStatistics) -> &AtomicUsize,
    ) -> Self {
        Self { writer, filter, stats, rejected_counter }
    }
}

impl<T: IteratorWriter<Entry>, F: EntryFilter> IteratorWriter<Entry> for FilteredOutputWriter<T, F> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        let mut filter = self.filter;
        let stats = Arc::clone(&self.stats);
        let rejected_counter = self.rejected_counter;

        let filtered_entries = entries.filter(move |entry| {
            let accepted = filter.accept(entry);
            if !accepted {
                rejected_counter(&stats).fetch_add(1, Ordering::Relaxed);
            }
            accepted
        });

        self.writer.write(filtered_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::output::statistics::OutputStatistics;
    use crate::output::writers::fixtures::CollectingWriter;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_filtered_writer_deduplicates_entries() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let filter = DuplicateEntryFilter::try_from(config::DuplicateFilter {
            match_on: vec![config::OutputFields::File, config::OutputFields::Directory],
        })
        .unwrap();

        let sut = FilteredOutputWriter::new(writer, filter, Arc::clone(&stats), |s| &s.duplicates_detected);

        let entries = vec![
            Entry::from_arguments_str("file1.c", vec!["gcc", "-c"], "/project", None),
            Entry::from_arguments_str("file1.c", vec!["gcc", "-c", "-Wall"], "/project", None),
            Entry::from_arguments_str("file2.c", vec!["gcc", "-c"], "/project", None),
            Entry::from_arguments_str("file1.c", vec!["gcc", "-c", "-O2"], "/project", None),
        ];

        sut.write(entries.into_iter()).unwrap();

        let result = collected.lock().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file, std::path::PathBuf::from("file1.c"));
        assert_eq!(result[1].file, std::path::PathBuf::from("file2.c"));
        assert_eq!(stats.duplicates_detected.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_filtered_writer_filters_by_source() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let filter = SourceEntryFilter::from(config::SourceFilter {
            directories: vec![
                config::DirectoryRule {
                    path: std::path::PathBuf::from("src"),
                    action: config::DirectoryAction::Include,
                },
                config::DirectoryRule {
                    path: std::path::PathBuf::from("/usr/include"),
                    action: config::DirectoryAction::Exclude,
                },
            ],
        });

        let sut =
            FilteredOutputWriter::new(writer, filter, Arc::clone(&stats), |s| &s.entries_filtered_by_source);

        let entries = vec![
            Entry::from_arguments_str("src/main.c", vec!["gcc", "-c"], "/project", None),
            Entry::from_arguments_str("/usr/include/stdio.h", vec!["gcc", "-c"], "/project", None),
            Entry::from_arguments_str("lib/utils.c", vec!["gcc", "-c"], "/project", None),
        ];

        sut.write(entries.into_iter()).unwrap();

        let result = collected.lock().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file, std::path::PathBuf::from("src/main.c"));
        assert_eq!(result[1].file, std::path::PathBuf::from("lib/utils.c"));
        assert_eq!(stats.entries_filtered_by_source.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_filtered_writer_preserves_order() {
        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let filter = DuplicateEntryFilter::try_from(config::DuplicateFilter {
            match_on: vec![config::OutputFields::File],
        })
        .unwrap();

        let sut = FilteredOutputWriter::new(writer, filter, Arc::clone(&stats), |s| &s.duplicates_detected);

        let entries = vec![
            Entry::from_arguments_str("c.c", vec!["gcc", "-c"], "/project", None),
            Entry::from_arguments_str("a.c", vec!["gcc", "-c"], "/project", None),
            Entry::from_arguments_str("b.c", vec!["gcc", "-c"], "/project", None),
        ];

        sut.write(entries.into_iter()).unwrap();

        let result = collected.lock().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].file, std::path::PathBuf::from("c.c"));
        assert_eq!(result[1].file, std::path::PathBuf::from("a.c"));
        assert_eq!(result[2].file, std::path::PathBuf::from("b.c"));
    }
}

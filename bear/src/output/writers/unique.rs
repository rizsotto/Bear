// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang;
use crate::output::statistics::OutputStatistics;
use crate::{config, output::WriterCreationError};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The type represents a writer that filters duplicate compilation database entries.
///
/// # Features
/// - Filters duplicates based on the provided configuration.
pub(crate) struct UniqueOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    filter: clang::DuplicateEntryFilter,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> UniqueOutputWriter<T> {
    pub(crate) fn create(
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

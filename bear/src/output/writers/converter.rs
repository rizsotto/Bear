// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang;
use crate::output::statistics::OutputStatistics;
use crate::{config, semantic};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The type represents a converter that formats `semantic::Command` instances into `Entry` objects.
pub(crate) struct ConverterClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    converter: clang::CommandConverter,
    writer: T,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> ConverterClangOutputWriter<T> {
    pub(crate) fn new(writer: T, format: &config::Format, stats: Arc<OutputStatistics>) -> Self {
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

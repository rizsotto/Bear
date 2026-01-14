// SPDX-License-Identifier: GPL-3.0-or-later

//! Statistics collection for the output pipeline.
//!
//! This module provides a centralized statistics structure that tracks metrics
//! across all stages of the output pipeline. Each writer in the chain updates
//! its specific field(s) as entries flow through, enabling comprehensive
//! visibility into the pipeline's behavior.
//!
//! # Usage
//!
//! Create a shared `OutputStatistics` instance and pass it to each writer
//! during pipeline construction. After processing completes, the statistics
//! can be logged to provide insight into how entries were transformed,
//! filtered, or deduplicated at each stage.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Statistics collected during the output pipeline execution.
///
/// Each writer in the chain updates its specific field(s) using atomic
/// operations, allowing for lock-free updates as entries flow through.
///
/// # Fields by Writer
///
/// - **ConverterClangOutputWriter**: `semantic_commands_received`, `compilation_entries_produced`
/// - **AppendClangOutputWriter**: `entries_read_from_existing`
/// - **UniqueOutputWriter**: `duplicates_detected`
/// - **SourceFilterOutputWriter**: `entries_filtered_by_source`
/// - **ClangOutputWriter**: `entries_written`
#[derive(Debug, Default)]
pub struct OutputStatistics {
    /// Number of semantic commands received by the converter.
    pub semantic_commands_received: AtomicUsize,

    /// Number of compilation database entries produced by the converter.
    pub compilation_entries_produced: AtomicUsize,

    /// Number of entries read from an existing compilation database (append mode).
    pub entries_read_from_existing: AtomicUsize,

    /// Number of duplicate entries detected and removed.
    pub duplicates_detected: AtomicUsize,

    /// Number of entries filtered out based on source file location rules.
    pub entries_filtered_by_source: AtomicUsize,

    /// Total number of entries written to the final output file.
    pub entries_written: AtomicUsize,
}

impl OutputStatistics {
    /// Creates a new `OutputStatistics` instance wrapped in an `Arc` for sharing.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl fmt::Display for OutputStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let semantic = self.semantic_commands_received.load(Ordering::Relaxed);
        let produced = self.compilation_entries_produced.load(Ordering::Relaxed);
        let from_existing = self.entries_read_from_existing.load(Ordering::Relaxed);
        let duplicates = self.duplicates_detected.load(Ordering::Relaxed);
        let filtered = self.entries_filtered_by_source.load(Ordering::Relaxed);
        let written = self.entries_written.load(Ordering::Relaxed);

        writeln!(f, "Output pipeline:")?;
        writeln!(f, "  semantic events: {}", semantic)?;
        writeln!(f, "  current entries: {}", produced)?;
        writeln!(f, "  previous entries: {}", from_existing)?;
        writeln!(f, "  filtered entries by duplicate: {}", duplicates)?;
        writeln!(f, "  filtered entries by source: {}", filtered)?;
        write!(f, "  total entries written: {}", written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_arc() {
        let stats = OutputStatistics::new();
        assert_eq!(stats.semantic_commands_received.load(Ordering::Relaxed), 0);
        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_display_format() {
        let stats = OutputStatistics::new();
        stats.semantic_commands_received.store(20, Ordering::Relaxed);
        stats.compilation_entries_produced.store(15, Ordering::Relaxed);
        stats.entries_read_from_existing.store(5, Ordering::Relaxed);
        stats.duplicates_detected.store(3, Ordering::Relaxed);
        stats.entries_filtered_by_source.store(2, Ordering::Relaxed);
        stats.entries_written.store(10, Ordering::Relaxed);

        let output = format!("{}", stats);
        assert!(output.contains("Output pipeline:"));
        assert!(output.contains("semantic events: 20"));
        assert!(output.contains("current entries: 15"));
        assert!(output.contains("previous entries: 5"));
        assert!(output.contains("filtered entries by duplicate: 3"));
        assert!(output.contains("filtered entries by source: 2"));
        assert!(output.contains("total entries written: 10"));
    }

    #[test]
    fn test_atomic_updates_from_multiple_references() {
        let stats = OutputStatistics::new();
        let stats_clone = Arc::clone(&stats);

        stats.semantic_commands_received.fetch_add(5, Ordering::Relaxed);
        stats_clone.semantic_commands_received.fetch_add(3, Ordering::Relaxed);

        assert_eq!(stats.semantic_commands_received.load(Ordering::Relaxed), 8);
    }
}

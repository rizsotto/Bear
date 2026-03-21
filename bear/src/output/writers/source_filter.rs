// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::WriterError;
use crate::output::clang;
use crate::output::statistics::OutputStatistics;
use crate::{config, output::WriterCreationError};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// The type represents a writer that filters compilation database entries based on source file paths.
///
/// # Features
/// - Filters entries based on directory-based rules with order-based evaluation semantics.
/// - Uses the configured source filter to include/exclude files.
pub(crate) struct SourceFilterOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    filter: clang::SourceEntryFilter,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<clang::Entry>> SourceFilterOutputWriter<T> {
    pub(crate) fn create(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryAction, DirectoryRule, SourceFilter};
    use crate::output::writers::MockWriter;
    use crate::output::writers::file::ClangOutputWriter;
    use crate::output::writers::unique::UniqueOutputWriter;
    use std::path::PathBuf;

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
        let dir = tempfile::tempdir().unwrap();
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
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("src/main.c"));
        assert!(content.contains("lib/utils.c"));
        assert!(content.contains("src/helper.c"));
        assert!(!content.contains("/usr/include/stdio.h")); // This should be filtered out
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! Source filtering for the output pipeline.
//!
//! This module provides both the source file filtering logic and the pipeline writer
//! that uses it. The filtering system allows fine-grained control over which source
//! files appear in the generated `compile_commands.json` while maintaining predictable
//! and explicit behavior.
//!
//! ## Evaluation Strategy
//!
//! The source filter uses the following evaluation strategy:
//!
//! 1. **Order-based evaluation**: Rules are processed in the order they appear in the
//!    configuration. For each source file, the filter iterates through all rules and
//!    applies the **last** matching rule's action.
//!
//! 2. **Empty directories list**: If no directory rules are configured, all files are
//!    included (no filtering is applied).
//!
//! 3. **No-match behavior**: If no rule matches a file, the file is **included** by default.
//!
//! 4. **Path matching**: Uses simple prefix matching with `Path::starts_with()`. A rule
//!    matches a file if the file's path starts with the rule's path.
//!
//! 5. **Case sensitivity**: Path matching is always case-sensitive on all platforms.
//!
//! 6. **No normalization**: Performs no path normalization or canonicalization during
//!    matching. Paths are compared as literal strings.
//!
//! 7. **Directory vs. file matching**: A directory rule matches both files directly in
//!    that directory and files in any subdirectory (recursive matching).

use super::IteratorWriter;
use crate::config::{DirectoryAction, SourceFilter};
use crate::output::WriterError;
use crate::output::clang::Entry;
use crate::output::statistics::OutputStatistics;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

// --- Source entry filter ---

/// A filter that determines which compilation database entries should be included
/// based on source file paths and directory-based rules.
#[derive(Debug)]
pub(crate) struct SourceEntryFilter {
    /// The source filter configuration containing directory rules.
    config: SourceFilter,
}

impl SourceEntryFilter {
    /// Determines whether a compilation database entry should be included.
    pub(crate) fn should_include(&self, entry: &Entry) -> bool {
        self.should_include_path(&entry.file)
    }

    /// Determines whether a file path should be included based on the configured rules.
    pub(crate) fn should_include_path(&self, file_path: &Path) -> bool {
        // Empty directories list means include everything
        if self.config.directories.is_empty() {
            return true;
        }

        let mut result = true; // Default: include if no rule matches

        // Order-based evaluation: last matching rule wins
        for rule in &self.config.directories {
            if file_path.starts_with(&rule.path) {
                result = match rule.action {
                    DirectoryAction::Include => true,
                    DirectoryAction::Exclude => false,
                };
            }
        }

        result
    }
}

impl From<SourceFilter> for SourceEntryFilter {
    fn from(config: SourceFilter) -> Self {
        SourceEntryFilter { config }
    }
}

// --- Pipeline writer ---

/// The type represents a writer that filters compilation database entries based on source file paths.
///
/// # Features
/// - Filters entries based on directory-based rules with order-based evaluation semantics.
/// - Uses the configured source filter to include/exclude files.
pub(crate) struct SourceFilterOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    filter: SourceEntryFilter,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<Entry>> SourceFilterOutputWriter<T> {
    pub(crate) fn new(writer: T, config: SourceFilter, stats: Arc<OutputStatistics>) -> Self {
        let filter = SourceEntryFilter::from(config);
        Self { writer, filter, stats }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for SourceFilterOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
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
    use crate::output::clang;
    use crate::output::writers::MockWriter;
    use crate::output::writers::file::ClangOutputWriter;
    use crate::output::writers::unique::UniqueOutputWriter;
    use std::path::PathBuf;

    // --- Source entry filter tests ---

    fn create_test_entry(file_path: &str, directory: &str) -> Entry {
        Entry::from_arguments_str(file_path, vec!["gcc", "-c"], directory, None)
    }

    /// Test helper function to filter a collection of compilation database entries
    fn filter_entries<I>(filter: &SourceEntryFilter, entries: I) -> Vec<Entry>
    where
        I: IntoIterator<Item = Entry>,
    {
        entries.into_iter().filter(|entry| filter.should_include(entry)).collect()
    }

    #[test]
    fn test_empty_directories_includes_all() {
        let config = SourceFilter { directories: vec![] };
        let filter = SourceEntryFilter::from(config);

        let entry1 = create_test_entry("any/path.c", "/project");
        let entry2 = create_test_entry("/absolute/path.cpp", "/project");
        let entry3 = create_test_entry("src/main.rs", "/project");

        assert!(filter.should_include(&entry1));
        assert!(filter.should_include(&entry2));
        assert!(filter.should_include(&entry3));
    }

    #[test]
    fn test_order_based_evaluation() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("src/test"), action: DirectoryAction::Exclude },
                DirectoryRule {
                    path: PathBuf::from("src/test/integration"),
                    action: DirectoryAction::Include,
                },
            ],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
        assert!(!filter.should_include(&create_test_entry("src/test/unit.c", "/project")));
        assert!(!filter.should_include(&create_test_entry("src/test/mock.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/test/integration/api.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/test/integration/db.c", "/project")));
    }

    #[test]
    fn test_no_match_behavior() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(filter.should_include(&create_test_entry("lib/external.c", "/project")));
        assert!(filter.should_include(&create_test_entry("vendor/third_party.cpp", "/project")));
        assert!(filter.should_include(&create_test_entry("/opt/custom/tool.c", "/project")));
    }

    #[test]
    fn test_exact_path_matching() {
        let config = SourceFilter {
            directories: vec![DirectoryRule {
                path: PathBuf::from("src/main.c"),
                action: DirectoryAction::Exclude,
            }],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(!filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/main.cpp", "/project")));
        assert!(filter.should_include(&create_test_entry("src/main.c.backup", "/project")));
        assert!(filter.should_include(&create_test_entry("src/main_test.c", "/project")));
    }

    #[test]
    fn test_prefix_matching() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include }],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/deeply/nested/file.c", "/project")));
        assert!(filter.should_include(&create_test_entry("not_src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("prefix_src/main.c", "/project")));
    }

    #[cfg(unix)]
    #[test]
    fn test_case_sensitivity_unix() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Exclude }],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(!filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("Src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("SRC/main.c", "/project")));
    }

    #[cfg(windows)]
    #[test]
    fn test_case_sensitivity_windows() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Exclude }],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(!filter.should_include(&create_test_entry("src\\main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("Src\\main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("SRC\\main.c", "/project")));
    }

    #[test]
    fn test_platform_separators() {
        #[cfg(unix)]
        let config = SourceFilter {
            directories: vec![DirectoryRule {
                path: PathBuf::from("src/lib"),
                action: DirectoryAction::Exclude,
            }],
        };

        #[cfg(windows)]
        let config = SourceFilter {
            directories: vec![DirectoryRule {
                path: PathBuf::from("src\\lib"),
                action: DirectoryAction::Exclude,
            }],
        };

        let filter = SourceEntryFilter::from(config);

        #[cfg(unix)]
        {
            assert!(!filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
            assert!(filter.should_include(&create_test_entry("src\\lib\\utils.c", "/project")));
        }

        #[cfg(windows)]
        {
            assert!(!filter.should_include(&create_test_entry("src\\lib\\utils.c", "/project")));
            assert!(!filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
            assert!(filter.should_include(&create_test_entry("other/path/utils.c", "/project")));
        }
    }

    #[test]
    fn test_complex_scenario() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("."), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
                DirectoryRule { path: PathBuf::from("/usr/local/include"), action: DirectoryAction::Exclude },
                DirectoryRule { path: PathBuf::from("build"), action: DirectoryAction::Exclude },
                DirectoryRule { path: PathBuf::from("target"), action: DirectoryAction::Exclude },
                DirectoryRule { path: PathBuf::from("build/config"), action: DirectoryAction::Include },
            ],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(filter.should_include(&create_test_entry("./src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("./lib/utils.c", "/project")));
        assert!(!filter.should_include(&create_test_entry("/usr/include/stdio.h", "/project")));
        assert!(
            !filter.should_include(&create_test_entry("/usr/local/include/boost/algorithm.hpp", "/project"))
        );
        assert!(!filter.should_include(&create_test_entry("build/main.o", "/project")));
        assert!(!filter.should_include(&create_test_entry("target/release/app", "/project")));
        assert!(filter.should_include(&create_test_entry("build/config/settings.h", "/project")));
        assert!(filter.should_include(&create_test_entry("build/config/generated/defs.h", "/project")));
    }

    #[test]
    fn test_filter_entries_method() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr"), action: DirectoryAction::Exclude },
            ],
        };
        let filter = SourceEntryFilter::from(config);

        let entries = vec![
            create_test_entry("src/main.c", "/project"),
            create_test_entry("/usr/include/stdio.h", "/project"),
            create_test_entry("lib/utils.c", "/project"),
            create_test_entry("src/helper.c", "/project"),
        ];

        let filtered = filter_entries(&filter, entries);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].file, PathBuf::from("src/main.c"));
        assert_eq!(filtered[1].file, PathBuf::from("lib/utils.c"));
        assert_eq!(filtered[2].file, PathBuf::from("src/helper.c"));
    }

    #[test]
    fn test_should_include_path_method() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include }],
        };
        let filter = SourceEntryFilter::from(config);

        assert!(filter.should_include_path(Path::new("src/main.c")));
        assert!(filter.should_include_path(Path::new("lib/utils.c")));
        assert!(filter.should_include_path(Path::new("other/file.c")));
    }

    // --- Pipeline writer tests with CollectingWriter ---

    #[test]
    fn test_source_filter_with_collecting_writer_verifies_entries() {
        use crate::output::writers::CollectingWriter;
        use std::sync::atomic::Ordering;

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };

        let sut = SourceFilterOutputWriter::new(writer, config, Arc::clone(&stats));

        let entries = vec![
            clang::Entry::from_arguments_str("src/main.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("/usr/include/stdio.h", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("lib/utils.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("src/helper.c", vec!["gcc", "-c"], "/project", None),
        ];

        sut.write(entries.into_iter()).unwrap();

        let result = collected.lock().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].file, PathBuf::from("src/main.c"));
        assert_eq!(result[1].file, PathBuf::from("lib/utils.c"));
        assert_eq!(result[2].file, PathBuf::from("src/helper.c"));
        assert_eq!(stats.entries_filtered_by_source.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_source_filter_with_collecting_writer_empty_config_passes_all() {
        use crate::output::writers::CollectingWriter;
        use std::sync::atomic::Ordering;

        let stats = OutputStatistics::new();
        let (writer, collected) = CollectingWriter::new();
        let config = SourceFilter::default();

        let sut = SourceFilterOutputWriter::new(writer, config, Arc::clone(&stats));

        let entries = vec![
            clang::Entry::from_arguments_str("any/file.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("/usr/include/stdio.h", vec!["gcc", "-c"], "/project", None),
        ];

        sut.write(entries.into_iter()).unwrap();

        let result = collected.lock().unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(stats.entries_filtered_by_source.load(Ordering::Relaxed), 0);
    }

    // --- Pipeline writer tests ---

    #[test]
    fn test_source_filter_output_writer_includes_matching_entries() {
        let stats = OutputStatistics::new();
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };

        let sut = SourceFilterOutputWriter::new(MockWriter, config, Arc::clone(&stats));

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

        let sut = SourceFilterOutputWriter::new(MockWriter, config, Arc::clone(&stats));

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

        let sut = SourceFilterOutputWriter::new(MockWriter, config, Arc::clone(&stats));

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

        let source_config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };

        let duplicate_config = crate::config::DuplicateFilter {
            match_on: vec![crate::config::OutputFields::File, crate::config::OutputFields::Directory],
        };

        let base_writer = ClangOutputWriter::create(&output_path, Arc::clone(&stats)).unwrap();
        let unique_writer =
            UniqueOutputWriter::create(base_writer, duplicate_config, Arc::clone(&stats)).unwrap();
        let source_filter_writer =
            SourceFilterOutputWriter::new(unique_writer, source_config, Arc::clone(&stats));

        let entries = vec![
            clang::Entry::from_arguments_str("src/main.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("/usr/include/stdio.h", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("lib/utils.c", vec!["gcc", "-c"], "/project", None),
            clang::Entry::from_arguments_str("src/helper.c", vec!["gcc", "-c"], "/project", None),
        ];

        assert!(source_filter_writer.write(entries.into_iter()).is_ok());

        assert!(output_path.exists());
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("src/main.c"));
        assert!(content.contains("lib/utils.c"));
        assert!(content.contains("src/helper.c"));
        assert!(!content.contains("/usr/include/stdio.h"));
    }
}

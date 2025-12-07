// SPDX-License-Identifier: GPL-3.0-or-later

//! Source filtering for compilation database entries.
//!
//! This module provides functionality to filter compilation database entries based on
//! directory-based rules with order-based evaluation semantics. The filtering system
//! allows fine-grained control over which source files appear in the generated
//! `compile_commands.json` while maintaining predictable and explicit behavior.
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

use std::path::Path;

use super::Entry;
use crate::config::{DirectoryAction, SourceFilter};
use thiserror::Error;

/// A filter that determines which compilation database entries should be included
/// based on source file paths and directory-based rules.
#[derive(Clone, Debug)]
pub struct SourceEntryFilter {
    /// The source filter configuration containing directory rules.
    config: SourceFilter,
}

impl SourceEntryFilter {
    /// Determines whether a compilation database entry should be included.
    ///
    /// This method evaluates the entry's file path against the configured directory rules
    /// using order-based evaluation semantics.
    ///
    /// # Arguments
    ///
    /// * `entry` - The compilation database entry to evaluate
    ///
    /// # Returns
    ///
    /// `true` if the entry should be included, `false` if it should be excluded
    ///
    /// # Evaluation Rules
    ///
    /// - If `directories` is empty, returns `true` (include everything)
    /// - Iterates through rules in order, updating result when path prefix matches
    /// - The *last* matching rule determines the final result
    /// - If no rule matches, returns `true` (include by default)
    pub fn should_include(&self, entry: &Entry) -> bool {
        self.should_include_path(&entry.file)
    }

    /// Determines whether a file path should be included based on the configured rules.
    ///
    /// This is the core evaluation logic that can also be used independently of
    /// compilation database entries.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path of the source file to evaluate
    ///
    /// # Returns
    ///
    /// `true` if the file should be included, `false` if it should be excluded
    pub fn should_include_path(&self, file_path: &Path) -> bool {
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

/// Represents configuration errors that can occur when creating a source filter.
#[derive(Error, Debug, PartialEq)]
pub enum SourceFilterError {
    /// No errors currently defined, but this allows for future extensibility
    /// without breaking changes to the public API.
    #[error("Configuration validation failed: {message}")]
    ConfigurationError { message: String },
}

impl TryFrom<SourceFilter> for SourceEntryFilter {
    type Error = SourceFilterError;

    fn try_from(config: SourceFilter) -> Result<Self, Self::Error> {
        // Currently no validation errors are possible since the config validation
        // happens at the configuration level, but this structure allows for future
        // enhancements without breaking the API.
        Ok(SourceEntryFilter { config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryAction, DirectoryRule};
    use std::path::PathBuf;

    /// Test helper function to filter a collection of compilation database entries
    fn filter_entries<I>(filter: &SourceEntryFilter, entries: I) -> Vec<Entry>
    where
        I: IntoIterator<Item = Entry>,
    {
        entries
            .into_iter()
            .filter(|entry| filter.should_include(entry))
            .collect()
    }

    fn create_test_entry(file_path: &str, directory: &str) -> Entry {
        Entry::from_arguments_str(file_path, vec!["gcc", "-c"], directory, None)
    }

    #[test]
    fn test_empty_directories_includes_all() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

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
            only_existing_files: true,
            directories: vec![
                DirectoryRule {
                    path: PathBuf::from("src"),
                    action: DirectoryAction::Include,
                },
                DirectoryRule {
                    path: PathBuf::from("src/test"),
                    action: DirectoryAction::Exclude,
                },
                DirectoryRule {
                    path: PathBuf::from("src/test/integration"),
                    action: DirectoryAction::Include,
                },
            ],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Files in src are included (first rule)
        assert!(filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));

        // Files in src/test are excluded (second rule overrides first)
        assert!(!filter.should_include(&create_test_entry("src/test/unit.c", "/project")));
        assert!(!filter.should_include(&create_test_entry("src/test/mock.c", "/project")));

        // Files in src/test/integration are included (third rule overrides second)
        assert!(filter.should_include(&create_test_entry("src/test/integration/api.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/test/integration/db.c", "/project")));
    }

    #[test]
    fn test_no_match_behavior() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![
                DirectoryRule {
                    path: PathBuf::from("src"),
                    action: DirectoryAction::Include,
                },
                DirectoryRule {
                    path: PathBuf::from("/usr/include"),
                    action: DirectoryAction::Exclude,
                },
            ],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Files that don't match any rule are included by default
        assert!(filter.should_include(&create_test_entry("lib/external.c", "/project")));
        assert!(filter.should_include(&create_test_entry("vendor/third_party.cpp", "/project")));
        assert!(filter.should_include(&create_test_entry("/opt/custom/tool.c", "/project")));
    }

    #[test]
    fn test_exact_path_matching() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src/main.c"),
                action: DirectoryAction::Exclude,
            }],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Exact file match
        assert!(!filter.should_include(&create_test_entry("src/main.c", "/project")));

        // Similar but different files are not affected
        assert!(filter.should_include(&create_test_entry("src/main.cpp", "/project")));
        assert!(filter.should_include(&create_test_entry("src/main.c.backup", "/project")));
        assert!(filter.should_include(&create_test_entry("src/main_test.c", "/project")));
    }

    #[test]
    fn test_prefix_matching() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src"),
                action: DirectoryAction::Include,
            }],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Directory prefix matches
        assert!(filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
        assert!(filter.should_include(&create_test_entry("src/deeply/nested/file.c", "/project")));

        // Non-prefix matches don't work
        assert!(filter.should_include(&create_test_entry("not_src/main.c", "/project"))); // default include
        assert!(filter.should_include(&create_test_entry("prefix_src/main.c", "/project")));
        // default include
    }

    #[cfg(unix)]
    #[test]
    fn test_case_sensitivity_unix() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src"),
                action: DirectoryAction::Exclude,
            }],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Case-sensitive matching on Unix
        assert!(!filter.should_include(&create_test_entry("src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("Src/main.c", "/project"))); // default include
        assert!(filter.should_include(&create_test_entry("SRC/main.c", "/project")));
        // default include
    }

    #[cfg(windows)]
    #[test]
    fn test_case_sensitivity_windows() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src"),
                action: DirectoryAction::Exclude,
            }],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Case-sensitive matching on Windows (even though filesystem is case-insensitive)
        assert!(!filter.should_include(&create_test_entry("src\\main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("Src\\main.c", "/project"))); // default include
        assert!(filter.should_include(&create_test_entry("SRC\\main.c", "/project")));
        // default include
    }

    #[test]
    fn test_platform_separators() {
        #[cfg(unix)]
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src/lib"),
                action: DirectoryAction::Exclude,
            }],
        };

        #[cfg(windows)]
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src\\lib"),
                action: DirectoryAction::Exclude,
            }],
        };

        let filter = SourceEntryFilter::try_from(config).unwrap();

        #[cfg(unix)]
        {
            // On Unix, only forward slash paths match the forward slash rule
            assert!(!filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
            // Backslash paths don't match on Unix - default include
            assert!(filter.should_include(&create_test_entry("src\\lib\\utils.c", "/project")));
        }

        #[cfg(windows)]
        {
            // On Windows, both separators are normalized and should match the rule
            assert!(!filter.should_include(&create_test_entry("src\\lib\\utils.c", "/project")));
            assert!(!filter.should_include(&create_test_entry("src/lib/utils.c", "/project")));
            // Test a path that doesn't match the rule
            assert!(filter.should_include(&create_test_entry("other/path/utils.c", "/project")));
        }
    }

    #[test]
    fn test_complex_scenario() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![
                // Include everything in project
                DirectoryRule {
                    path: PathBuf::from("."),
                    action: DirectoryAction::Include,
                },
                // Exclude system headers
                DirectoryRule {
                    path: PathBuf::from("/usr/include"),
                    action: DirectoryAction::Exclude,
                },
                DirectoryRule {
                    path: PathBuf::from("/usr/local/include"),
                    action: DirectoryAction::Exclude,
                },
                // Exclude build artifacts
                DirectoryRule {
                    path: PathBuf::from("build"),
                    action: DirectoryAction::Exclude,
                },
                DirectoryRule {
                    path: PathBuf::from("target"),
                    action: DirectoryAction::Exclude,
                },
                // But include specific build config files
                DirectoryRule {
                    path: PathBuf::from("build/config"),
                    action: DirectoryAction::Include,
                },
            ],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        // Project files are included
        assert!(filter.should_include(&create_test_entry("./src/main.c", "/project")));
        assert!(filter.should_include(&create_test_entry("./lib/utils.c", "/project")));

        // System headers are excluded
        assert!(!filter.should_include(&create_test_entry("/usr/include/stdio.h", "/project")));
        assert!(!filter.should_include(&create_test_entry(
            "/usr/local/include/boost/algorithm.hpp",
            "/project"
        )));

        // Build artifacts are excluded
        assert!(!filter.should_include(&create_test_entry("build/main.o", "/project")));
        assert!(!filter.should_include(&create_test_entry("target/release/app", "/project")));

        // But specific build config is included (last rule wins)
        assert!(filter.should_include(&create_test_entry("build/config/settings.h", "/project")));
        assert!(filter.should_include(&create_test_entry(
            "build/config/generated/defs.h",
            "/project"
        )));
    }

    #[test]
    fn test_filter_entries_method() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![
                DirectoryRule {
                    path: PathBuf::from("src"),
                    action: DirectoryAction::Include,
                },
                DirectoryRule {
                    path: PathBuf::from("/usr"),
                    action: DirectoryAction::Exclude,
                },
            ],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        let entries = vec![
            create_test_entry("src/main.c", "/project"), // should be included
            create_test_entry("/usr/include/stdio.h", "/project"), // should be excluded
            create_test_entry("lib/utils.c", "/project"), // should be included (no match)
            create_test_entry("src/helper.c", "/project"), // should be included
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
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src"),
                action: DirectoryAction::Include,
            }],
        };
        let filter = SourceEntryFilter::try_from(config).unwrap();

        assert!(filter.should_include_path(Path::new("src/main.c")));
        assert!(filter.should_include_path(Path::new("lib/utils.c"))); // no match = include
        assert!(filter.should_include_path(Path::new("other/file.c"))); // no match = include
    }

    #[test]
    fn test_try_from_success() {
        let config = SourceFilter {
            only_existing_files: true,
            directories: vec![DirectoryRule {
                path: PathBuf::from("src"),
                action: DirectoryAction::Include,
            }],
        };

        let result = SourceEntryFilter::try_from(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_empty_config() {
        let config = SourceFilter {
            only_existing_files: false,
            directories: vec![],
        };

        let result = SourceEntryFilter::try_from(config);
        assert!(result.is_ok());
    }
}

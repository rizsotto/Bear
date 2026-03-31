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

use crate::config::{DirectoryAction, SourceFilter};
use crate::output::clang::Entry;
use std::path::Path;

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
    fn should_include(&self, entry: &Entry) -> bool {
        self.should_include_path(&entry.file)
    }

    /// Determines whether a file path should be included based on the configured rules.
    fn should_include_path(&self, file_path: &Path) -> bool {
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

impl super::EntryFilter for SourceEntryFilter {
    fn accept(&mut self, entry: &Entry) -> bool {
        self.should_include(entry)
    }
}

impl From<SourceFilter> for SourceEntryFilter {
    fn from(config: SourceFilter) -> Self {
        SourceEntryFilter { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryAction, DirectoryRule, SourceFilter};
    use crate::output::writers::filtering::EntryFilter;
    use std::path::PathBuf;

    fn create_test_entry(file_path: &str, directory: &str) -> Entry {
        Entry::from_arguments_str(file_path, vec!["gcc", "-c"], directory, None)
    }

    #[test]
    fn test_empty_directories_accepts_all() {
        let config = SourceFilter { directories: vec![] };
        let mut filter = SourceEntryFilter::from(config);

        assert!(filter.accept(&create_test_entry("any/path.c", "/project")));
        assert!(filter.accept(&create_test_entry("/absolute/path.cpp", "/project")));
        assert!(filter.accept(&create_test_entry("src/main.rs", "/project")));
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
        let mut filter = SourceEntryFilter::from(config);

        assert!(filter.accept(&create_test_entry("src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("src/lib/utils.c", "/project")));
        assert!(!filter.accept(&create_test_entry("src/test/unit.c", "/project")));
        assert!(!filter.accept(&create_test_entry("src/test/mock.c", "/project")));
        assert!(filter.accept(&create_test_entry("src/test/integration/api.c", "/project")));
        assert!(filter.accept(&create_test_entry("src/test/integration/db.c", "/project")));
    }

    #[test]
    fn test_no_match_accepts() {
        let config = SourceFilter {
            directories: vec![
                DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include },
                DirectoryRule { path: PathBuf::from("/usr/include"), action: DirectoryAction::Exclude },
            ],
        };
        let mut filter = SourceEntryFilter::from(config);

        assert!(filter.accept(&create_test_entry("lib/external.c", "/project")));
        assert!(filter.accept(&create_test_entry("vendor/third_party.cpp", "/project")));
        assert!(filter.accept(&create_test_entry("/opt/custom/tool.c", "/project")));
    }

    #[test]
    fn test_exact_path_matching() {
        let config = SourceFilter {
            directories: vec![DirectoryRule {
                path: PathBuf::from("src/main.c"),
                action: DirectoryAction::Exclude,
            }],
        };
        let mut filter = SourceEntryFilter::from(config);

        assert!(!filter.accept(&create_test_entry("src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("src/main.cpp", "/project")));
        assert!(filter.accept(&create_test_entry("src/main.c.backup", "/project")));
        assert!(filter.accept(&create_test_entry("src/main_test.c", "/project")));
    }

    #[test]
    fn test_prefix_matching() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include }],
        };
        let mut filter = SourceEntryFilter::from(config);

        assert!(filter.accept(&create_test_entry("src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("src/lib/utils.c", "/project")));
        assert!(filter.accept(&create_test_entry("src/deeply/nested/file.c", "/project")));
        assert!(filter.accept(&create_test_entry("not_src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("prefix_src/main.c", "/project")));
    }

    #[cfg(unix)]
    #[test]
    fn test_case_sensitivity_unix() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Exclude }],
        };
        let mut filter = SourceEntryFilter::from(config);

        assert!(!filter.accept(&create_test_entry("src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("Src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("SRC/main.c", "/project")));
    }

    #[cfg(windows)]
    #[test]
    fn test_case_sensitivity_windows() {
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Exclude }],
        };
        let mut filter = SourceEntryFilter::from(config);

        assert!(!filter.accept(&create_test_entry("src\\main.c", "/project")));
        assert!(filter.accept(&create_test_entry("Src\\main.c", "/project")));
        assert!(filter.accept(&create_test_entry("SRC\\main.c", "/project")));
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

        let mut filter = SourceEntryFilter::from(config);

        #[cfg(unix)]
        {
            assert!(!filter.accept(&create_test_entry("src/lib/utils.c", "/project")));
            assert!(filter.accept(&create_test_entry("src\\lib\\utils.c", "/project")));
        }

        #[cfg(windows)]
        {
            assert!(!filter.accept(&create_test_entry("src\\lib\\utils.c", "/project")));
            assert!(!filter.accept(&create_test_entry("src/lib/utils.c", "/project")));
            assert!(filter.accept(&create_test_entry("other/path/utils.c", "/project")));
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
        let mut filter = SourceEntryFilter::from(config);

        assert!(filter.accept(&create_test_entry("./src/main.c", "/project")));
        assert!(filter.accept(&create_test_entry("./lib/utils.c", "/project")));
        assert!(!filter.accept(&create_test_entry("/usr/include/stdio.h", "/project")));
        assert!(!filter.accept(&create_test_entry("/usr/local/include/boost/algorithm.hpp", "/project")));
        assert!(!filter.accept(&create_test_entry("build/main.o", "/project")));
        assert!(!filter.accept(&create_test_entry("target/release/app", "/project")));
        assert!(filter.accept(&create_test_entry("build/config/settings.h", "/project")));
        assert!(filter.accept(&create_test_entry("build/config/generated/defs.h", "/project")));
    }
}

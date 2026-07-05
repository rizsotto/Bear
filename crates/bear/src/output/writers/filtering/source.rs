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
//!
//! ## Filename-pattern rules
//!
//! Alongside directory rules, the source filter supports filename-glob rules that target
//! machine-generated sources (Qt `moc` output, protobuf stubs, and similar):
//!
//! 1. **Basename vs. full-path matching**: A pattern with no path separator (`/`, or `\` on
//!    Windows) matches the source file's basename; a pattern containing a separator matches
//!    the full source path as it appears in the entry.
//! 2. **Order-based evaluation**: Evaluated independently of the directory rules, using the
//!    same last-match-wins semantics: the last rule whose glob matches determines the verdict.
//! 3. **Empty files list**: Interpreted as "include everything" (no filtering).
//! 4. **No-match behavior**: If no pattern matches a file, the file is **included**.
//! 5. **Composition**: An entry is emitted only when both the directory rules and the
//!    filename-pattern rules accept it (a logical AND of the two independent verdicts).
//! 6. **Compilation**: Each `glob::Pattern` is compiled once, when the filter is built from
//!    configuration, not per entry. Configuration validation rejects patterns that fail to
//!    compile before a filter is ever built from them.

use crate::config::{DirectoryAction, SourceFilter};
use crate::output::clang::Entry;
use std::path::Path;

// --- Source entry filter ---

/// A filename-glob rule compiled once, at filter construction.
#[derive(Debug)]
struct CompiledFileRule {
    /// The compiled glob pattern.
    pattern: glob::Pattern,
    /// The action to apply when the pattern matches.
    action: DirectoryAction,
    /// Whether the pattern matches the full source path (`true`) or just the basename
    /// (`false`), decided once from whether the original pattern text contains a path
    /// separator.
    match_full_path: bool,
}

/// A filter that determines which compilation database entries should be included
/// based on source file paths, directory-based rules, and filename-glob rules.
#[derive(Debug)]
pub(crate) struct SourceEntryFilter {
    /// The source filter configuration containing directory rules.
    config: SourceFilter,
    /// Filename-glob rules, compiled once from `config.files`.
    file_rules: Vec<CompiledFileRule>,
}

impl SourceEntryFilter {
    /// Determines whether a compilation database entry should be included.
    fn should_include(&self, entry: &Entry) -> bool {
        self.should_include_path(&entry.file)
    }

    /// Determines whether a file path should be included based on the configured rules.
    ///
    /// An entry is included only when both the directory rules and the filename-pattern
    /// rules accept it.
    fn should_include_path(&self, file_path: &Path) -> bool {
        self.directories_accept(file_path) && self.files_accept(file_path)
    }

    /// Evaluates the directory rules for a file path.
    fn directories_accept(&self, file_path: &Path) -> bool {
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

    /// Evaluates the filename-glob rules for a file path.
    fn files_accept(&self, file_path: &Path) -> bool {
        // Empty files list means include everything
        if self.file_rules.is_empty() {
            return true;
        }

        // Computed once, reused across the rule list below.
        let full_path = file_path.to_string_lossy();
        let basename = file_path.file_name().map(|name| name.to_string_lossy());

        let mut result = true; // Default: include if no rule matches

        // Order-based evaluation: last matching rule wins
        for rule in &self.file_rules {
            let candidate: &str = if rule.match_full_path {
                &full_path
            } else {
                match &basename {
                    Some(name) => name,
                    // No basename (e.g. a path ending in a separator): a basename rule
                    // has nothing to compare against, so it cannot match.
                    None => continue,
                }
            };

            if rule.pattern.matches(candidate) {
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
        let file_rules = config
            .files
            .iter()
            .map(|rule| {
                let match_full_path =
                    rule.pattern.contains('/') || rule.pattern.contains(std::path::MAIN_SEPARATOR);
                let pattern = glob::Pattern::new(&rule.pattern)
                    .expect("config validation rejects patterns that do not compile");
                CompiledFileRule { pattern, action: rule.action, match_full_path }
            })
            .collect();

        SourceEntryFilter { config, file_rules }
    }
}

// Requirements: output-source-directory-filter
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DirectoryAction, DirectoryRule, FileRule, SourceFilter};
    use crate::output::writers::filtering::EntryFilter;
    use std::path::PathBuf;

    fn create_test_entry(file_path: &str, directory: &str) -> Entry {
        Entry::from_arguments_str(file_path, vec!["gcc", "-c"], directory, None)
    }

    #[test]
    fn test_empty_directories_accepts_all() {
        let config = SourceFilter { directories: vec![], files: vec![] };
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
            files: vec![],
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
            files: vec![],
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
            files: vec![],
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
            files: vec![],
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
            files: vec![],
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
            files: vec![],
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
            files: vec![],
        };

        #[cfg(windows)]
        let config = SourceFilter {
            directories: vec![DirectoryRule {
                path: PathBuf::from("src\\lib"),
                action: DirectoryAction::Exclude,
            }],
            files: vec![],
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
            files: vec![],
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

    // Requirements: output-generated-file-filter
    #[test]
    fn test_file_pattern_rules() {
        struct Case {
            name: &'static str,
            files: Vec<FileRule>,
            path: &'static str,
            expected: bool,
        }

        fn rule(pattern: &str, action: DirectoryAction) -> FileRule {
            FileRule { pattern: pattern.to_string(), action }
        }

        let cases = vec![
            Case {
                name: "basename exclude drops a matching generated file",
                files: vec![rule("moc_*.cpp", DirectoryAction::Exclude)],
                path: "src/moc_window.cpp",
                expected: false,
            },
            Case {
                name: "basename exclude leaves a non-matching file untouched",
                files: vec![rule("moc_*.cpp", DirectoryAction::Exclude)],
                path: "src/main.cpp",
                expected: true,
            },
            Case {
                name: "last-match-wins re-includes after a broader exclude",
                files: vec![
                    rule("*.cpp", DirectoryAction::Exclude),
                    rule("main.cpp", DirectoryAction::Include),
                ],
                path: "src/main.cpp",
                expected: true,
            },
            Case {
                name: "a pattern with a separator matches the full path",
                files: vec![rule("generated/*.cpp", DirectoryAction::Exclude)],
                path: "generated/moc_window.cpp",
                expected: false,
            },
            Case {
                name: "a pattern with a separator does not match by basename alone",
                files: vec![rule("generated/*.cpp", DirectoryAction::Exclude)],
                path: "src/moc_window.cpp",
                expected: true,
            },
            Case {
                name: "a file matched by no pattern rule is included",
                files: vec![rule("moc_*.cpp", DirectoryAction::Exclude)],
                path: "src/util.cpp",
                expected: true,
            },
        ];

        for case in cases {
            let config = SourceFilter { directories: vec![], files: case.files };
            let mut sut = SourceEntryFilter::from(config);

            let actual = sut.accept(&create_test_entry(case.path, "/project"));

            assert_eq!(actual, case.expected, "case: {}", case.name);
        }
    }

    // Requirements: output-generated-file-filter
    #[test]
    fn test_directory_and_file_rules_compose() {
        // Accepted by the directory rules, excluded by a file-pattern rule: dropped.
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include }],
            files: vec![FileRule { pattern: "moc_*.cpp".to_string(), action: DirectoryAction::Exclude }],
        };
        let mut sut = SourceEntryFilter::from(config);
        assert!(!sut.accept(&create_test_entry("src/moc_window.cpp", "/project")));

        // Excluded by the directory rules, accepted by the file-pattern rules: still dropped.
        let config = SourceFilter {
            directories: vec![DirectoryRule {
                path: PathBuf::from("build"),
                action: DirectoryAction::Exclude,
            }],
            files: vec![FileRule { pattern: "*.cpp".to_string(), action: DirectoryAction::Include }],
        };
        let mut sut = SourceEntryFilter::from(config);
        assert!(!sut.accept(&create_test_entry("build/main.cpp", "/project")));

        // Accepted by both rule sets: included.
        let config = SourceFilter {
            directories: vec![DirectoryRule { path: PathBuf::from("src"), action: DirectoryAction::Include }],
            files: vec![FileRule { pattern: "moc_*.cpp".to_string(), action: DirectoryAction::Exclude }],
        };
        let mut sut = SourceEntryFilter::from(config);
        assert!(sut.accept(&create_test_entry("src/main.cpp", "/project")));
    }

    #[cfg(windows)]
    // Requirements: output-generated-file-filter
    #[test]
    fn test_file_pattern_windows_separator_matches_full_path() {
        let config = SourceFilter {
            directories: vec![],
            files: vec![FileRule {
                pattern: "generated\\moc_window.cpp".to_string(),
                action: DirectoryAction::Exclude,
            }],
        };
        let mut sut = SourceEntryFilter::from(config);

        assert!(!sut.accept(&create_test_entry("generated\\moc_window.cpp", "/project")));
        assert!(sut.accept(&create_test_entry("other\\moc_window.cpp", "/project")));
    }
}

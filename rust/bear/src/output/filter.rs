// SPDX-License-Identifier: GPL-3.0-or-later

use std::hash::Hash;
use std::path::Path;

use crate::config;
use crate::output::clang::Entry;
use builder::create_hash;
use builder::EntryPredicateBuilder as Builder;

/// A predicate that can be used to filter compilation database entries.
///
/// If the predicate returns `true`, the entry is included in the result set.
/// If the predicate returns `false`, the entry is excluded from the result set.
pub type EntryPredicate = Box<dyn FnMut(&Entry) -> bool>;

impl From<&config::SourceFilter> for EntryPredicate {
    /// Create a filter from the configuration.
    fn from(config: &config::SourceFilter) -> Self {
        let source_exist_check = Builder::filter_by_source_existence(config.only_existing_files);

        let mut source_path_checks = Builder::new();
        for config::DirectoryFilter { path, ignore } in &config.paths {
            let filter = Builder::filter_by_source_path(path);
            match ignore {
                config::Ignore::Always => {
                    source_path_checks = source_path_checks & !filter;
                }
                config::Ignore::Never => {
                    source_path_checks = source_path_checks & filter;
                }
            }
        }

        (source_exist_check & source_path_checks).build()
    }
}

impl From<&config::DuplicateFilter> for EntryPredicate {
    /// Create a filter from the configuration.
    fn from(config: &config::DuplicateFilter) -> Self {
        let hash_function = create_hash(&config.by_fields);
        Builder::filter_duplicate_entries(hash_function).build()
    }
}

mod builder {
    use super::*;
    use std::collections::HashSet;
    use std::hash::{DefaultHasher, Hasher};

    /// Represents a builder object that can be used to construct an entry predicate.
    pub(super) struct EntryPredicateBuilder {
        candidate: Option<EntryPredicate>,
    }

    impl EntryPredicateBuilder {
        /// Creates an entry predicate from the builder.
        pub(super) fn build(self) -> EntryPredicate {
            match self.candidate {
                Some(predicate) => predicate,
                None => Box::new(|_: &Entry| true),
            }
        }

        /// Construct a predicate builder that is empty.
        #[inline]
        pub(crate) fn new() -> Self {
            Self { candidate: None }
        }

        /// Construct a predicate builder that implements a predicate.
        #[inline]
        fn from<P>(predicate: P) -> Self
        where
            P: FnMut(&Entry) -> bool + 'static,
        {
            Self {
                candidate: Some(Box::new(predicate)),
            }
        }

        /// Create a predicate that filters out entries
        /// that are not using any of the given source paths.
        pub(super) fn filter_by_source_path(path: &Path) -> Self {
            let owned_path = path.to_owned();
            Self::from(move |entry| entry.file.starts_with(owned_path.clone()))
        }

        /// Create a predicate that filters out entries
        /// that source file does not exist.
        pub(super) fn filter_by_source_existence(only_existing: bool) -> Self {
            if only_existing {
                Self::from(|entry| entry.file.is_file())
            } else {
                Self::new()
            }
        }

        /// Create a predicate that filters out entries
        /// that are already in the compilation database based on their hash.
        pub(super) fn filter_duplicate_entries(
            hash_function: impl Fn(&Entry) -> u64 + 'static,
        ) -> Self {
            let mut have_seen = HashSet::new();

            Self::from(move |entry| {
                let hash = hash_function(entry);
                if !have_seen.contains(&hash) {
                    have_seen.insert(hash);
                    true
                } else {
                    false
                }
            })
        }
    }

    /// Implement the AND operator for combining predicates.
    impl std::ops::BitAnd for EntryPredicateBuilder {
        type Output = EntryPredicateBuilder;

        fn bitand(self, rhs: Self) -> Self::Output {
            match (self.candidate, rhs.candidate) {
                (None, None) => EntryPredicateBuilder::new(),
                (None, some) => EntryPredicateBuilder { candidate: some },
                (some, None) => EntryPredicateBuilder { candidate: some },
                (Some(mut lhs), Some(mut rhs)) => EntryPredicateBuilder::from(move |entry| {
                    let result = lhs(entry);
                    if result {
                        rhs(entry)
                    } else {
                        result
                    }
                }),
            }
        }
    }

    /// Implement the NOT operator for combining predicates.
    impl std::ops::Not for EntryPredicateBuilder {
        type Output = EntryPredicateBuilder;

        fn not(self) -> Self::Output {
            match self.candidate {
                Some(mut original) => Self::from(move |entry| {
                    let result = original(entry);
                    !result
                }),
                None => Self::new(),
            }
        }
    }

    /// Create a hash function that is using the given fields to calculate the hash of an entry.
    pub(super) fn create_hash(fields: &[config::OutputFields]) -> impl Fn(&Entry) -> u64 + 'static {
        let owned_fields: Vec<config::OutputFields> = fields.to_vec();
        move |entry: &Entry| {
            let mut hasher = DefaultHasher::new();
            for field in &owned_fields {
                match field {
                    config::OutputFields::Directory => entry.directory.hash(&mut hasher),
                    config::OutputFields::File => entry.file.hash(&mut hasher),
                    config::OutputFields::Arguments => entry.arguments.hash(&mut hasher),
                    config::OutputFields::Output => entry.output.hash(&mut hasher),
                }
            }
            hasher.finish()
        }
    }

    #[cfg(test)]
    mod sources_test {
        use super::*;
        use crate::vec_of_strings;
        use std::path::PathBuf;

        #[test]
        fn test_filter_by_source_paths() {
            let input: Vec<Entry> = vec![
                Entry {
                    file: PathBuf::from("/home/user/project/source/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
                Entry {
                    file: PathBuf::from("/home/user/project/test/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "test.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
            ];

            let expected: Vec<Entry> = vec![input[0].clone()];

            let config = config::SourceFilter {
                only_existing_files: false,
                paths: vec![
                    config::DirectoryFilter {
                        path: PathBuf::from("/home/user/project/source"),
                        ignore: config::Ignore::Never,
                    },
                    config::DirectoryFilter {
                        path: PathBuf::from("/home/user/project/test"),
                        ignore: config::Ignore::Always,
                    },
                ],
            };
            let sut: EntryPredicate = From::from(&config);
            let result: Vec<Entry> = input.into_iter().filter(sut).collect();
            assert_eq!(result, expected);
        }
    }

    #[cfg(test)]
    mod existence_test {
        use super::*;
        use crate::vec_of_strings;
        use std::hash::{Hash, Hasher};
        use std::path::PathBuf;

        #[test]
        fn test_duplicate_detection_works() {
            let input: Vec<Entry> = vec![
                Entry {
                    file: PathBuf::from("/home/user/project/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/source.o")),
                },
                Entry {
                    file: PathBuf::from("/home/user/project/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "-Wall", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/source.o")),
                },
                Entry {
                    file: PathBuf::from("/home/user/project/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c", "-o", "test.o"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/test.o")),
                },
            ];

            let expected: Vec<Entry> = vec![input[0].clone(), input[2].clone()];

            let hash_function = |entry: &Entry| {
                let mut hasher = DefaultHasher::new();
                entry.file.hash(&mut hasher);
                entry.output.hash(&mut hasher);
                hasher.finish()
            };
            let sut: EntryPredicate =
                EntryPredicateBuilder::filter_duplicate_entries(hash_function).build();
            let result: Vec<Entry> = input.into_iter().filter(sut).collect();
            assert_eq!(result, expected);
        }
    }

    #[cfg(test)]
    mod create_hash_tests {
        use super::*;
        use crate::vec_of_strings;
        use std::path::PathBuf;

        #[test]
        fn test_create_hash_with_directory_field() {
            let entry = create_test_entry();

            let fields = vec![config::OutputFields::Directory];
            let hash_function = create_hash(&fields);
            let hash = hash_function(&entry);

            let mut hasher = DefaultHasher::new();
            entry.directory.hash(&mut hasher);
            let expected_hash = hasher.finish();

            assert_eq!(hash, expected_hash);
        }

        #[test]
        fn test_create_hash_with_file_field() {
            let entry = create_test_entry();

            let fields = vec![config::OutputFields::File];
            let hash_function = create_hash(&fields);
            let hash = hash_function(&entry);

            let mut hasher = DefaultHasher::new();
            entry.file.hash(&mut hasher);
            let expected_hash = hasher.finish();

            assert_eq!(hash, expected_hash);
        }

        #[test]
        fn test_create_hash_with_arguments_field() {
            let entry = create_test_entry();

            let fields = vec![config::OutputFields::Arguments];
            let hash_function = create_hash(&fields);
            let hash = hash_function(&entry);

            let mut hasher = DefaultHasher::new();
            entry.arguments.hash(&mut hasher);
            let expected_hash = hasher.finish();

            assert_eq!(hash, expected_hash);
        }

        #[test]
        fn test_create_hash_with_output_field() {
            let entry = create_test_entry();

            let fields = vec![config::OutputFields::Output];
            let hash_function = create_hash(&fields);
            let hash = hash_function(&entry);

            let mut hasher = DefaultHasher::new();
            entry.output.hash(&mut hasher);
            let expected_hash = hasher.finish();

            assert_eq!(hash, expected_hash);
        }

        #[test]
        fn test_create_hash_with_multiple_fields() {
            let entry = create_test_entry();

            let fields = vec![
                config::OutputFields::Directory,
                config::OutputFields::File,
                config::OutputFields::Arguments,
                config::OutputFields::Output,
            ];
            let hash_function = create_hash(&fields);
            let hash = hash_function(&entry);

            let mut hasher = DefaultHasher::new();
            entry.directory.hash(&mut hasher);
            entry.file.hash(&mut hasher);
            entry.arguments.hash(&mut hasher);
            entry.output.hash(&mut hasher);
            let expected_hash = hasher.finish();

            assert_eq!(hash, expected_hash);
        }

        fn create_test_entry() -> Entry {
            Entry {
                file: PathBuf::from("/home/user/project/source.c"),
                arguments: vec_of_strings!["cc", "-c", "source.c"],
                directory: PathBuf::from("/home/user/project"),
                output: Some(PathBuf::from("/home/user/project/source.o")),
            }
        }
    }

    #[cfg(test)]
    mod bitand_tests {
        use super::*;
        use crate::vec_of_strings;
        use std::path::PathBuf;

        #[test]
        fn test_bitand_both_predicates_true() {
            let input = create_test_entries();

            let predicate1 = EntryPredicateBuilder::from(|_: &Entry| true);
            let predicate2 = EntryPredicateBuilder::from(|_: &Entry| true);
            let combined_predicate = (predicate1 & predicate2).build();

            let result: Vec<Entry> = input.into_iter().filter(combined_predicate).collect();
            assert_eq!(result.len(), 1);
        }

        #[test]
        fn test_bitand_first_predicate_false() {
            let input = create_test_entries();

            let predicate1 = EntryPredicateBuilder::from(|_: &Entry| false);
            let predicate2 = EntryPredicateBuilder::from(|_: &Entry| true);
            let combined_predicate = (predicate1 & predicate2).build();

            let result: Vec<Entry> = input.into_iter().filter(combined_predicate).collect();
            assert_eq!(result.len(), 0);
        }

        #[test]
        fn test_bitand_second_predicate_false() {
            let input = create_test_entries();

            let predicate1 = EntryPredicateBuilder::from(|_: &Entry| true);
            let predicate2 = EntryPredicateBuilder::from(|_: &Entry| false);
            let combined_predicate = (predicate1 & predicate2).build();

            let result: Vec<Entry> = input.into_iter().filter(combined_predicate).collect();
            assert_eq!(result.len(), 0);
        }

        #[test]
        fn test_bitand_both_predicates_false() {
            let input = create_test_entries();

            let predicate1 = EntryPredicateBuilder::from(|_: &Entry| false);
            let predicate2 = EntryPredicateBuilder::from(|_: &Entry| false);
            let combined_predicate = (predicate1 & predicate2).build();

            let result: Vec<Entry> = input.into_iter().filter(combined_predicate).collect();
            assert_eq!(result.len(), 0);
        }

        fn create_test_entries() -> Vec<Entry> {
            vec![Entry {
                file: PathBuf::from("/home/user/project/source/source.c"),
                arguments: vec_of_strings!["cc", "-c", "source.c"],
                directory: PathBuf::from("/home/user/project"),
                output: None,
            }]
        }
    }

    #[cfg(test)]
    mod not_tests {
        use super::*;
        use crate::vec_of_strings;
        use std::path::PathBuf;

        #[test]
        fn test_not_predicate_true() {
            let input = create_test_entries();

            let predicate = EntryPredicateBuilder::from(|_: &Entry| true);
            let not_predicate = (!predicate).build();

            let result: Vec<Entry> = input.into_iter().filter(not_predicate).collect();
            assert_eq!(result.len(), 0);
        }

        #[test]
        fn test_not_predicate_false() {
            let input = create_test_entries();

            let predicate = EntryPredicateBuilder::from(|_: &Entry| false);
            let not_predicate = (!predicate).build();

            let result: Vec<Entry> = input.into_iter().filter(not_predicate).collect();
            assert_eq!(result.len(), 1);
        }

        fn create_test_entries() -> Vec<Entry> {
            vec![Entry {
                file: PathBuf::from("/home/user/project/source/source.c"),
                arguments: vec_of_strings!["cc", "-c", "source.c"],
                directory: PathBuf::from("/home/user/project"),
                output: None,
            }]
        }
    }
}

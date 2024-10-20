// SPDX-License-Identifier: GPL-3.0-or-later
use std::hash::Hash;
use std::path::PathBuf;

use super::clang::Entry;
use super::config;
use builder::create_hash;
use builder::EntryPredicateBuilder as Builder;

/// A predicate that can be used to filter compilation database entries.
///
/// If the predicate returns `true`, the entry is included in the result set.
/// If the predicate returns `false`, the entry is excluded from the result set.
pub type EntryPredicate = Box<dyn FnMut(&Entry) -> bool>;

impl TryFrom<&config::Filter> for EntryPredicate {
    type Error = anyhow::Error;

    /// Create a filter from the configuration.
    fn try_from(config: &config::Filter) -> Result<Self, Self::Error> {
        // - Check if the source file exists
        // - Check if the source file is not in the exclude list of the configuration
        // - Check if the source file is in the include list of the configuration
        let source_exist_check =
            Builder::filter_by_source_existence(config.source.include_only_existing_files);
        let source_paths_to_exclude =
            Builder::filter_by_source_paths(&config.source.paths_to_exclude);
        let source_paths_to_include =
            Builder::filter_by_source_paths(&config.source.paths_to_include);
        let source_checks = source_exist_check & !source_paths_to_exclude & source_paths_to_include;
        // - Check if the compiler path is not in the list of the configuration
        // - Check if the compiler arguments are not in the list of the configuration
        let compiler_with_path = Builder::filter_by_compiler_paths(&config.compilers.with_paths);
        let compiler_with_argument =
            Builder::filter_by_compiler_arguments(&config.compilers.with_arguments);
        let compiler_checks = !compiler_with_path & !compiler_with_argument;
        // - Check if the entry is not a duplicate based on the fields of the configuration
        let hash_function = create_hash(&config.duplicates.by_fields);
        let duplicates = Builder::filter_duplicate_entries(hash_function);

        Ok((source_checks & compiler_checks & duplicates).build())
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
        fn new() -> Self {
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
        /// that are using one of the given compilers.
        pub(super) fn filter_by_compiler_paths(paths: &[PathBuf]) -> Self {
            if paths.is_empty() {
                Self::new()
            } else {
                let owned_paths: Vec<PathBuf> = paths.iter().cloned().collect();
                Self::from(move |entry| {
                    let compiler = PathBuf::from(entry.arguments[0].as_str());
                    // return true if none of the paths are a prefix of the compiler path.
                    owned_paths.iter().any(|path| !compiler.starts_with(path))
                })
            }
        }

        /// Create a predicate that filters out entries
        /// that are using one of the given compiler arguments.
        pub(super) fn filter_by_compiler_arguments(flags: &[String]) -> Self {
            if flags.is_empty() {
                Self::new()
            } else {
                let owned_flags: HashSet<String> = flags.iter().cloned().collect();
                Self::from(move |entry| {
                    let mut arguments = entry.arguments.iter().skip(1);
                    // return true if none of the flags are in the arguments.
                    arguments.all(|argument| !owned_flags.contains(argument))
                })
            }
        }

        /// Create a predicate that filters out entries
        /// that are not using any of the given source paths.
        pub(super) fn filter_by_source_paths(paths: &[PathBuf]) -> Self {
            if paths.is_empty() {
                Self::new()
            } else {
                let owned_paths: Vec<PathBuf> = paths.iter().cloned().collect();
                Self::from(move |entry| owned_paths.iter().any(|path| entry.file.starts_with(path)))
            }
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

    // FIXME: write unit tests for the combination operators.
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

    // FIXME: write unit tests for the combination operators.
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

    // FIXME: write unit tests for the hash function.
    /// Create a hash function that is using the given fields to calculate the hash of an entry.
    pub(super) fn create_hash(fields: &[config::OutputFields]) -> impl Fn(&Entry) -> u64 + 'static {
        let owned_fields: Vec<config::OutputFields> = fields.iter().cloned().collect();
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
    mod test {
        use super::*;
        use crate::{vec_of_pathbuf, vec_of_strings};
        use std::hash::{Hash, Hasher};

        #[test]
        fn test_filter_by_compiler_paths() {
            let input: Vec<Entry> = vec![
                Entry {
                    file: PathBuf::from("/home/user/project/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
                Entry {
                    file: PathBuf::from("/home/user/project/source.c++"),
                    arguments: vec_of_strings!["c++", "-c", "source.c++"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
                Entry {
                    file: PathBuf::from("/home/user/project/test.c"),
                    arguments: vec_of_strings!["cc", "-c", "test.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
            ];

            let expected: Vec<Entry> = vec![input[0].clone(), input[2].clone()];

            let sut: EntryPredicate =
                EntryPredicateBuilder::filter_by_compiler_paths(&vec_of_pathbuf!["c++"]).build();
            let result: Vec<Entry> = input.into_iter().filter(sut).collect();
            assert_eq!(result, expected);
        }

        #[test]
        fn test_filter_by_compiler_arguments() {
            let input: Vec<Entry> = vec![
                Entry {
                    file: PathBuf::from("/home/user/project/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
                Entry {
                    file: PathBuf::from("/home/user/project/source.c"),
                    arguments: vec_of_strings!["cc", "-cc1", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
                Entry {
                    file: PathBuf::from("/home/user/project/test.c"),
                    arguments: vec_of_strings!["cc", "-c", "test.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: None,
                },
            ];

            let expected: Vec<Entry> = vec![input[0].clone(), input[2].clone()];

            let sut: EntryPredicate =
                EntryPredicateBuilder::filter_by_compiler_arguments(&vec_of_strings!["-cc1"])
                    .build();
            let result: Vec<Entry> = input.into_iter().filter(sut).collect();
            assert_eq!(result, expected);
        }

        #[test]
        fn test_filter_by_source_paths() {
            let paths_to_include = vec_of_pathbuf!["/home/user/project/source"];
            let paths_to_exclude = vec_of_pathbuf!["/home/user/project/test"];

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

            let sut: EntryPredicate =
                (EntryPredicateBuilder::filter_by_source_paths(&paths_to_include)
                    & !EntryPredicateBuilder::filter_by_source_paths(&paths_to_exclude))
                .build();
            let result: Vec<Entry> = input.into_iter().filter(sut).collect();
            assert_eq!(result, expected);
        }

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
}

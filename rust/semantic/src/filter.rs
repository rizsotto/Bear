/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::collections::HashSet;
use std::path::PathBuf;

use json_compilation_db::Entry;

/// A predicate that can be used to filter compilation database entries.
///
/// If the predicate returns `true`, the entry is included in the result set.
/// If the predicate returns `false`, the entry is excluded from the result set.
pub type EntryPredicate = Box<dyn FnMut(&Entry) -> bool>;

/// Represents a builder object that can be used to construct an entry predicate.
///
/// The builder can be used to combine multiple predicates using logical operators.
pub struct EntryPredicateBuilder {
    candidate: Option<EntryPredicate>,
}

impl std::ops::BitAnd for EntryPredicateBuilder {
    type Output = EntryPredicateBuilder;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self.candidate, rhs.candidate) {
            (None, None) =>
                EntryPredicateBuilder::new(),
            (Some(mut lhs), Some(mut rhs)) => {
                EntryPredicateBuilder::from(move |entry| {
                    let result = lhs(entry);
                    if result {
                        rhs(entry)
                    } else {
                        result
                    }
                })
            }
            (None, some) =>
                EntryPredicateBuilder { candidate: some },
            (some, None) =>
                EntryPredicateBuilder { candidate: some },
        }
    }
}

impl std::ops::Not for EntryPredicateBuilder {
    type Output = EntryPredicateBuilder;

    fn not(self) -> Self::Output {
        match self.candidate {
            Some(mut original) => {
                Self::from(move |entry| {
                    let result = original(entry);
                    !result
                })
            }
            None =>
                Self::new(),
        }
    }
}

impl EntryPredicateBuilder {
    /// Creates an entry predicate from the builder.
    pub fn build(self) -> EntryPredicate {
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
        Self { candidate: Some(Box::new(predicate)) }
    }

    /// Create a predicate that filters out entries
    /// that are using one of the given compilers.
    pub fn filter_by_compiler_paths(paths: Vec<PathBuf>) -> Self {
        if paths.is_empty() {
            Self::new()
        } else {
            Self::from(move |entry| {
                let compiler = PathBuf::from(entry.arguments[0].as_str());
                // return true if none of the paths are a prefix of the compiler path.
                paths.iter().any(|path| { !compiler.starts_with(path) })
            })
        }
    }

    /// Create a predicate that filters out entries
    /// that are using one of the given compiler arguments.
    pub fn filter_by_compiler_arguments(flags: Vec<String>) -> Self {
        if flags.is_empty() {
            Self::new()
        } else {
            Self::from(move |entry| {
                let mut arguments = entry.arguments.iter().skip(1);
                // return true if none of the flags are in the arguments.
                arguments.all(|argument| { !flags.contains(&argument) })
            })
        }
    }

    /// Create a predicate that filters out entries
    /// that are not using any of the given source paths.
    pub fn filter_by_source_paths(paths: Vec<PathBuf>) -> Self {
        if paths.is_empty() {
            Self::new()
        } else {
            Self::from(move |entry| {
                paths.iter().any(|path| { entry.file.starts_with(path) })
            })
        }
    }

    /// Create a predicate that filters out entries
    /// that source file does not exist.
    pub fn filter_by_source_existence(only_existing: bool) -> Self {
        if only_existing {
            Self::from(|entry| { entry.file.is_file() })
        } else {
            Self::new()
        }
    }

    /// Create a predicate that filters out entries
    /// that are already in the compilation database based on their hash.
    pub fn filter_duplicate_entries(hash_function: impl Fn(&Entry) -> u64 + 'static) -> Self {
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

#[cfg(test)]
mod test {
    use std::hash::{Hash, Hasher};
    use crate::{vec_of_pathbuf, vec_of_strings};
    use super::*;

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

        let expected: Vec<Entry> = vec![
            input[0].clone(),
            input[2].clone(),
        ];

        let sut: EntryPredicate = EntryPredicateBuilder::filter_by_compiler_paths(vec_of_pathbuf!["c++"]).build();
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

        let expected: Vec<Entry> = vec![
            input[0].clone(),
            input[2].clone(),
        ];

        let sut: EntryPredicate = EntryPredicateBuilder::filter_by_compiler_arguments(vec_of_strings!["-cc1"]).build();
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

        let expected: Vec<Entry> = vec![
            input[0].clone(),
        ];

        let sut: EntryPredicate = (
            EntryPredicateBuilder::filter_by_source_paths(paths_to_include) &
                !EntryPredicateBuilder::filter_by_source_paths(paths_to_exclude)
        ).build();
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

        let expected: Vec<Entry> = vec![
            input[0].clone(),
            input[2].clone(),
        ];

        let hash_function = |entry: &Entry| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            entry.file.hash(&mut hasher);
            entry.output.hash(&mut hasher);
            hasher.finish()
        };
        let sut: EntryPredicate = EntryPredicateBuilder::filter_duplicate_entries(hash_function).build();
        let result: Vec<Entry> = input.into_iter().filter(sut).collect();
        assert_eq!(result, expected);
    }
}
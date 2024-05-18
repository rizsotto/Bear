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

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use json_compilation_db::Entry;

use crate::configuration::{Content, DuplicateFilterFields};

pub(crate) type EntryPredicate = Box<dyn FnMut(&Entry) -> bool>;

impl From<&Content> for EntryPredicate {
    fn from(val: &Content) -> Self {
        let source_check = EntryPredicateBuilder::source_check(val.include_only_existing_source);
        let paths_to_include = EntryPredicateBuilder::contains(val.paths_to_include.as_slice());
        let paths_to_exclude = EntryPredicateBuilder::contains(val.paths_to_exclude.as_slice());
        let duplicates = EntryPredicateBuilder::duplicates(&val.duplicate_filter_fields);

        (!paths_to_exclude & paths_to_include & source_check & duplicates).build()
    }
}


struct EntryPredicateBuilder {
    predicate_opt: Option<EntryPredicate>,
}

impl EntryPredicateBuilder {
    fn build(self) -> EntryPredicate {
        match self.predicate_opt {
            Some(predicate) => predicate,
            None => Box::new(|_: &Entry| true),
        }
    }

    fn source_check(include_only_existing_source: bool) -> Self {
        if include_only_existing_source {
            let predicate: EntryPredicate = Box::new(|entry| { entry.file.is_file() });
            EntryPredicateBuilder { predicate_opt: Some(predicate) }
        } else {
            EntryPredicateBuilder { predicate_opt: None }
        }
    }

    fn contains(paths: &[PathBuf]) -> Self {
        if paths.is_empty() {
            EntryPredicateBuilder { predicate_opt: None }
        } else {
            let paths_copy = paths.to_vec();
            let predicate: EntryPredicate = Box::new(move |entry| {
                paths_copy.iter().any(|path| { entry.file.starts_with(path) })
            });
            EntryPredicateBuilder { predicate_opt: Some(predicate) }
        }
    }

    fn duplicates(config: &DuplicateFilterFields) -> Self {
        let hash_function: fn(&Entry) -> u64 = config.into();
        let mut have_seen = HashSet::new();

        let predicate: EntryPredicate = Box::new(move |entry| {
            let hash = hash_function(entry);
            if !have_seen.contains(&hash) {
                have_seen.insert(hash);
                true
            } else {
                false
            }
        });
        EntryPredicateBuilder { predicate_opt: Some(predicate) }
    }
}

impl std::ops::BitAnd for EntryPredicateBuilder {
    type Output = EntryPredicateBuilder;

    fn bitand(self, rhs: Self) -> Self::Output {
        let predicate_opt = match (self.predicate_opt, rhs.predicate_opt) {
            (None, None) =>
                None,
            (Some(mut lhs), Some(mut rhs)) => {
                let predicate: EntryPredicate = Box::new(move |entry| {
                    let result = lhs(entry);
                    if result {
                        rhs(entry)
                    } else {
                        result
                    }
                });
                Some(predicate)
            }
            (None, some_predicate) =>
                some_predicate,
            (some_predicate, None) =>
                some_predicate,
        };
        EntryPredicateBuilder { predicate_opt }
    }
}

impl std::ops::Not for EntryPredicateBuilder {
    type Output = EntryPredicateBuilder;

    fn not(self) -> Self::Output {
        let predicate_opt = match self.predicate_opt {
            Some(mut original) => {
                let predicate: EntryPredicate = Box::new(move |entry| {
                    let result = original(entry);
                    !result
                });
                Some(predicate)
            }
            None =>
                None,
        };
        EntryPredicateBuilder { predicate_opt }
    }
}

impl DuplicateFilterFields {
    fn hash_source(entry: &Entry) -> u64 {
        let mut s = DefaultHasher::default();
        entry.file.hash(&mut s);
        s.finish()
    }

    fn hash_source_and_output(entry: &Entry) -> u64 {
        let mut s = DefaultHasher::default();
        entry.file.hash(&mut s);
        entry.output.hash(&mut s);
        s.finish()
    }

    fn hash_all(entry: &Entry) -> u64 {
        let mut s = DefaultHasher::default();
        entry.file.hash(&mut s);
        entry.directory.hash(&mut s);
        entry.arguments.hash(&mut s);
        s.finish()
    }
}

impl From<&DuplicateFilterFields> for fn(&Entry) -> u64 {
    fn from(val: &DuplicateFilterFields) -> Self {
        match val {
            DuplicateFilterFields::FileOnly =>
                DuplicateFilterFields::hash_source,
            DuplicateFilterFields::FileAndOutputOnly =>
                DuplicateFilterFields::hash_source_and_output,
            DuplicateFilterFields::All =>
                DuplicateFilterFields::hash_all,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{vec_of_pathbuf, vec_of_strings};
    use super::*;

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
                file: PathBuf::from("/home/user/project/test.c"),
                arguments: vec_of_strings!["cc", "-c", "test.c"],
                directory: PathBuf::from("/home/user/project"),
                output: Some(PathBuf::from("/home/user/project/test.o")),
            },
        ];

        let expected: Vec<Entry> = vec![
            Entry {
                file: PathBuf::from("/home/user/project/source.c"),
                arguments: vec_of_strings!["cc", "-c", "source.c"],
                directory: PathBuf::from("/home/user/project"),
                output: Some(PathBuf::from("/home/user/project/source.o")),
            },
            Entry {
                file: PathBuf::from("/home/user/project/test.c"),
                arguments: vec_of_strings!["cc", "-c", "test.c"],
                directory: PathBuf::from("/home/user/project"),
                output: Some(PathBuf::from("/home/user/project/test.o")),
            },
        ];

        let sut: EntryPredicate = (&Content::default()).into();
        let result: Vec<Entry> = input.into_iter().filter(sut).collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_exclude_include_works() {
        let configs: Vec<Content> = vec![
            Content {
                include_only_existing_source: false,
                duplicate_filter_fields: DuplicateFilterFields::default(),
                paths_to_include: vec_of_pathbuf!["/home/user/project/source"],
                paths_to_exclude: vec_of_pathbuf!["/home/user/project/test"],
            },
            Content {
                include_only_existing_source: false,
                duplicate_filter_fields: DuplicateFilterFields::default(),
                paths_to_include: vec_of_pathbuf!["/home/user/project/source/"],
                paths_to_exclude: vec_of_pathbuf!["/home/user/project/test/"],
            },
            Content {
                include_only_existing_source: false,
                duplicate_filter_fields: DuplicateFilterFields::default(),
                paths_to_include: vec_of_pathbuf!["/home/user/project"],
                paths_to_exclude: vec_of_pathbuf!["/home/user/project/test"],
            },
            Content {
                include_only_existing_source: false,
                duplicate_filter_fields: DuplicateFilterFields::default(),
                paths_to_include: vec_of_pathbuf!["/home/user/project/"],
                paths_to_exclude: vec_of_pathbuf!["/home/user/project/test/"],
            },
        ];

        for config in configs {
            let input: Vec<Entry> = vec![
                Entry {
                    file: PathBuf::from("/home/user/project/source/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/source/source.o")),
                },
                Entry {
                    file: PathBuf::from("/home/user/project/source/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "-Wall", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/source/source.o")),
                },
                Entry {
                    file: PathBuf::from("/home/user/project/test/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "test.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/test/source.o")),
                },
            ];

            let expected: Vec<Entry> = vec![
                Entry {
                    file: PathBuf::from("/home/user/project/source/source.c"),
                    arguments: vec_of_strings!["cc", "-c", "source.c"],
                    directory: PathBuf::from("/home/user/project"),
                    output: Some(PathBuf::from("/home/user/project/source/source.o")),
                },
            ];

            let sut: EntryPredicate = (&config).into();
            let result: Vec<Entry> = input.into_iter().filter(sut).collect();
            assert_eq!(expected, result);
        }
    }
}

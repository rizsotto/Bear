// SPDX-License-Identifier: GPL-3.0-or-later

//! A predicate that can be used to filter duplicate compilation database entries.
//!
//! The filter can be configured to use different fields of the compilation database
//! entries to determine if they are duplicates.

use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use super::Entry;
use crate::config;
use thiserror::Error;

#[derive(Clone, Debug)]
pub(crate) struct DuplicateFilter {
    /// The fields to use for filtering duplicate entries.
    fields: Vec<config::OutputFields>,
    /// The cache of seen hashes.
    hash_values: HashSet<u64>,
}

unsafe impl Send for DuplicateFilter {}

impl DuplicateFilter {
    pub fn unique(&mut self, entry: &Entry) -> bool {
        let hash = self.hash_function(entry);
        if self.hash_values.contains(&hash) {
            return false;
        }
        self.hash_values.insert(hash);
        true
    }

    fn hash_function(&self, entry: &Entry) -> u64 {
        let mut hasher = DefaultHasher::new();
        for field in &self.fields {
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

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Duplicate field: {0:?}")]
    DuplicateField(config::OutputFields),
    #[error("Empty field list")]
    EmptyFieldList,
}

impl TryFrom<config::DuplicateFilter> for DuplicateFilter {
    type Error = ConfigurationError;

    fn try_from(config: config::DuplicateFilter) -> Result<Self, Self::Error> {
        if config.by_fields.is_empty() {
            return Err(ConfigurationError::EmptyFieldList);
        }
        let mut already_seen = HashSet::new();
        for field in &config.by_fields {
            if !already_seen.insert(field) {
                return Err(ConfigurationError::DuplicateField(field.clone()));
            }
        }

        Ok(DuplicateFilter {
            fields: config.by_fields,
            hash_values: HashSet::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::entry;
    use super::*;

    #[test]
    fn test_try_from_success() {
        let config = config::DuplicateFilter {
            by_fields: vec![config::OutputFields::File, config::OutputFields::Directory],
        };

        let result = DuplicateFilter::try_from(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_failure_empty_fields() {
        let config = config::DuplicateFilter { by_fields: vec![] };

        let result = DuplicateFilter::try_from(config);
        assert!(matches!(result, Err(ConfigurationError::EmptyFieldList)));
    }

    #[test]
    fn test_try_from_failure_duplicate_fields() {
        let config = config::DuplicateFilter {
            by_fields: vec![config::OutputFields::File, config::OutputFields::File],
        };

        let result = DuplicateFilter::try_from(config);
        assert!(matches!(
            result,
            Err(ConfigurationError::DuplicateField(
                config::OutputFields::File
            ))
        ));
    }

    #[test]
    fn test_is_duplicate_with_file_and_directory_fields() {
        let config = config::DuplicateFilter {
            by_fields: vec![config::OutputFields::File, config::OutputFields::Directory],
        };
        let mut sut = DuplicateFilter::try_from(config).unwrap();

        let entry1 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "-Wall", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );

        assert!(sut.unique(&entry1));
        assert!(!sut.unique(&entry2));
    }

    #[test]
    fn test_is_duplicate_with_output_field() {
        let config = config::DuplicateFilter {
            by_fields: vec![config::OutputFields::Output],
        };
        let mut sut = DuplicateFilter::try_from(config).unwrap();

        let entry1 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c", "-o", "test.o"],
            "/home/user/project",
            Some("/home/user/project/test.o"),
        );

        assert!(sut.unique(&entry1));
        assert!(sut.unique(&entry2));
    }

    #[test]
    fn test_is_duplicate_with_arguments_field() {
        let config = config::DuplicateFilter {
            by_fields: vec![config::OutputFields::Arguments],
        };
        let mut sut = DuplicateFilter::try_from(config).unwrap();

        let entry1 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "-Wall", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );

        assert!(sut.unique(&entry1));
        assert!(sut.unique(&entry2));
    }

    #[test]
    fn test_is_unique() {
        let config = config::DuplicateFilter {
            by_fields: vec![config::OutputFields::File],
        };
        let mut sut = DuplicateFilter::try_from(config).unwrap();

        let entry1 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = entry(
            "/home/user/project/source.c",
            vec!["cc", "-c", "-Wall", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );

        assert!(sut.unique(&entry1));
        assert!(!sut.unique(&entry2));
    }
}

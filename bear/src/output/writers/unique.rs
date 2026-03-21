// SPDX-License-Identifier: GPL-3.0-or-later

//! Duplicate entry filtering for the output pipeline.
//!
//! This module provides both the duplicate detection logic and the pipeline writer
//! that uses it. The filter can be configured to use different fields of the
//! compilation database entries to determine if they are duplicates.

use super::IteratorWriter;
use crate::config;
use crate::output::WriterCreationError;
use crate::output::WriterError;
use crate::output::clang::Entry;
use crate::output::statistics::OutputStatistics;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use thiserror::Error;

// --- Duplicate entry filter ---

#[derive(Debug)]
pub(crate) struct DuplicateEntryFilter {
    /// The fields to use for filtering duplicate entries.
    fields: Vec<config::OutputFields>,
    /// The cache of seen hashes.
    hash_values: HashSet<u64>,
}

unsafe impl Send for DuplicateEntryFilter {}

impl DuplicateEntryFilter {
    pub(crate) fn unique(&mut self, entry: &Entry) -> bool {
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
                config::OutputFields::Command => entry.command.hash(&mut hasher),
                config::OutputFields::Output => entry.output.hash(&mut hasher),
            }
        }
        hasher.finish()
    }
}

#[derive(Error, Debug)]
pub(crate) enum ConfigurationError {
    #[error("Duplicate field: {0:?}")]
    DuplicateField(config::OutputFields),
    #[error("Command and Arguments cannot be both specified")]
    CommandAndArgumentsBothSpecified,
    #[error("Empty field list")]
    EmptyFieldList,
}

impl TryFrom<config::DuplicateFilter> for DuplicateEntryFilter {
    type Error = ConfigurationError;

    fn try_from(config: config::DuplicateFilter) -> Result<Self, Self::Error> {
        if config.match_on.is_empty() {
            return Err(ConfigurationError::EmptyFieldList);
        }
        let mut already_seen = HashSet::new();
        for field in &config.match_on {
            if !already_seen.insert(field) {
                return Err(ConfigurationError::DuplicateField(*field));
            }
        }

        if already_seen.contains(&config::OutputFields::Command)
            && already_seen.contains(&config::OutputFields::Arguments)
        {
            return Err(ConfigurationError::CommandAndArgumentsBothSpecified);
        }

        Ok(DuplicateEntryFilter { fields: config.match_on, hash_values: HashSet::new() })
    }
}

// --- Pipeline writer ---

/// The type represents a writer that filters duplicate compilation database entries.
///
/// # Features
/// - Filters duplicates based on the provided configuration.
pub(crate) struct UniqueOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    filter: DuplicateEntryFilter,
    stats: Arc<OutputStatistics>,
}

impl<T: IteratorWriter<Entry>> UniqueOutputWriter<T> {
    pub(crate) fn create(
        writer: T,
        config: config::DuplicateFilter,
        stats: Arc<OutputStatistics>,
    ) -> Result<Self, WriterCreationError> {
        let filter = DuplicateEntryFilter::try_from(config)
            .map_err(|err| WriterCreationError::Configuration(err.to_string()))?;

        Ok(Self { writer, filter, stats })
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for UniqueOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> Result<(), WriterError> {
        let mut filter = self.filter;
        let stats = Arc::clone(&self.stats);

        let filtered_entries = entries.filter(move |entry| {
            let is_unique = filter.unique(entry);
            if !is_unique {
                stats.duplicates_detected.fetch_add(1, Ordering::Relaxed);
            }
            is_unique
        });

        self.writer.write(filtered_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_success() {
        let config = config::DuplicateFilter {
            match_on: vec![config::OutputFields::File, config::OutputFields::Directory],
        };

        let result = DuplicateEntryFilter::try_from(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_failure_empty_fields() {
        let config = config::DuplicateFilter { match_on: vec![] };

        let result = DuplicateEntryFilter::try_from(config);
        assert!(matches!(result, Err(ConfigurationError::EmptyFieldList)));
    }

    #[test]
    fn test_try_from_failure_duplicate_fields() {
        let config = config::DuplicateFilter {
            match_on: vec![config::OutputFields::File, config::OutputFields::File],
        };

        let result = DuplicateEntryFilter::try_from(config);
        assert!(matches!(result, Err(ConfigurationError::DuplicateField(config::OutputFields::File))));
    }

    #[test]
    fn test_try_from_failure_command_and_arguments() {
        let config = config::DuplicateFilter {
            match_on: vec![config::OutputFields::Command, config::OutputFields::Arguments],
        };
        let result = DuplicateEntryFilter::try_from(config);
        assert!(matches!(result, Err(ConfigurationError::CommandAndArgumentsBothSpecified)));
    }

    #[test]
    fn test_is_duplicate_with_file_and_directory_fields() {
        let config = config::DuplicateFilter {
            match_on: vec![config::OutputFields::File, config::OutputFields::Directory],
        };
        let mut sut = DuplicateEntryFilter::try_from(config).unwrap();

        let entry1 = Entry::from_arguments_str(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = Entry::from_arguments_str(
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
        let config = config::DuplicateFilter { match_on: vec![config::OutputFields::Output] };
        let mut sut = DuplicateEntryFilter::try_from(config).unwrap();

        let entry1 = Entry::from_arguments_str(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = Entry::from_arguments_str(
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
        let config = config::DuplicateFilter { match_on: vec![config::OutputFields::Arguments] };
        let mut sut = DuplicateEntryFilter::try_from(config).unwrap();

        let entry1 = Entry::from_arguments_str(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = Entry::from_arguments_str(
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
        let config = config::DuplicateFilter { match_on: vec![config::OutputFields::File] };
        let mut sut = DuplicateEntryFilter::try_from(config).unwrap();

        let entry1 = Entry::from_arguments_str(
            "/home/user/project/source.c",
            vec!["cc", "-c", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );
        let entry2 = Entry::from_arguments_str(
            "/home/user/project/source.c",
            vec!["cc", "-c", "-Wall", "source.c"],
            "/home/user/project",
            Some("/home/user/project/source.o"),
        );

        assert!(sut.unique(&entry1));
        assert!(!sut.unique(&entry2));
    }
}

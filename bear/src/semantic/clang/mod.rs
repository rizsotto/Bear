// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides support for reading and writing JSON compilation database files.
//!
//! A compilation database is a set of records which describe the compilation of the
//! source files in a given project. It describes the compiler invocation command to
//! compile a source module to an object file.
//!
//! This database can have many forms. One well known and supported format is the JSON
//! compilation database, which is a simple JSON file having the list of compilation
//! as an array. The definition of the JSON compilation database files is done in the
//! LLVM project [documentation](https://clang.llvm.org/docs/JSONCompilationDatabase.html).

pub mod converter;
mod filter;
mod format;

use serde::{Deserialize, Serialize};
use shell_words;
use std::path;
use thiserror::Error;

// Re-export types for easier access
pub use converter::CommandConverter;
pub use filter::DuplicateEntryFilter;
pub use format::{FormatConfigurationError, FormatError, PathFormatter};

/// Represents an entry of the compilation database.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The main translation unit source processed by this compilation step.
    /// This is used by tools as the key into the compilation database.
    /// There can be multiple command objects for the same file, for example if the same
    /// source file is compiled with different configurations.
    file: path::PathBuf,
    /// The compile command argv as list of strings. This should run the compilation step
    /// for the translation unit file. `arguments[0]` should be the executable name, such
    /// as `clang++`. Arguments should not be escaped, but ready to pass to `execvp()`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    arguments: Vec<String>,
    /// The compile command as a single shell-escaped string. Arguments may be shell quoted
    /// and escaped following platform conventions, with ‘"’ and ‘\’ being the only special
    /// characters. Shell expansion is not supported.
    ///
    /// Either `arguments` or `command` is required. `arguments` is preferred, as shell
    /// (un)escaping is a possible source of errors.
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    command: String,
    /// The working directory of the compilation. All paths specified in the `command` or
    /// `file` fields must be either absolute or relative to this directory.
    directory: path::PathBuf,
    /// The name of the output created by this compilation step. This field is optional.
    /// It can be used to distinguish different processing modes of the same input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    output: Option<path::PathBuf>,
}

impl Entry {
    /// Create an Entry with the arguments field populated.
    pub fn with_arguments(
        file: impl Into<path::PathBuf>,
        arguments: Vec<String>,
        directory: impl Into<path::PathBuf>,
        output: Option<impl Into<path::PathBuf>>,
    ) -> Self {
        Entry {
            file: file.into(),
            arguments,
            command: String::default(),
            directory: directory.into(),
            output: output.map(|o| o.into()),
        }
    }

    /// Create an Entry with the command field populated.
    pub fn with_command(
        file: impl Into<path::PathBuf>,
        arguments: Vec<String>,
        directory: impl Into<path::PathBuf>,
        output: Option<impl Into<path::PathBuf>>,
    ) -> Self {
        Entry {
            file: file.into(),
            arguments: Vec::default(),
            command: shell_words::join(&arguments),
            directory: directory.into(),
            output: output.map(|o| o.into()),
        }
    }

    /// Semantic validation of the entry. Checking all fields for
    /// valid values and formats.
    pub fn validate(&self) -> Result<(), EntryError> {
        if self.file.to_string_lossy().is_empty() {
            return Err(EntryError::EmptyFileName);
        }
        if self.directory.to_string_lossy().is_empty() {
            return Err(EntryError::EmptyDirectory);
        }
        if self.command.is_empty() && self.arguments.is_empty() {
            return Err(EntryError::CommandOrArgumentsAreMissing);
        }
        if !self.command.is_empty() && !self.arguments.is_empty() {
            return Err(EntryError::CommandOrArgumentsArePresent);
        }
        if !self.command.is_empty() {
            shell_words::split(&self.command)?;
        }
        Ok(())
    }

    /// Constructor method for testing purposes.
    #[cfg(test)]
    pub fn from_arguments_str(
        file: &str,
        arguments: Vec<&str>,
        directory: &str,
        output: Option<&str>,
    ) -> Self {
        Self {
            file: file.into(),
            command: String::default(),
            arguments: arguments.into_iter().map(String::from).collect(),
            directory: directory.into(),
            output: output.map(|o| o.into()),
        }
    }

    /// Constructor method for testing purposes.
    #[cfg(test)]
    pub fn from_command_str(
        file: &str,
        command: &str,
        directory: &str,
        output: Option<&str>,
    ) -> Self {
        Self {
            file: file.into(),
            arguments: Vec::default(),
            command: command.into(),
            directory: directory.into(),
            output: output.map(|o| o.into()),
        }
    }
}

/// Represents the possible errors that can occur when validating an entry.
#[derive(Debug, Eq, PartialEq, Error)]
pub enum EntryError {
    #[error("Entry has an empty file field")]
    EmptyFileName,
    #[error("Entry has an empty directory field")]
    EmptyDirectory,
    #[error("Both command and arguments fields are empty")]
    CommandOrArgumentsAreMissing,
    #[error("Both command and arguments fields are present")]
    CommandOrArgumentsArePresent,
    #[error("Entry has an invalid command field: {0}")]
    InvalidCommand(#[from] shell_words::ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_validate_success_arguments() {
        let entry = Entry::from_arguments_str("main.cpp", vec!["clang", "-c"], "/tmp", None);

        assert!(entry.command.is_empty());
        assert!(!entry.arguments.is_empty());

        assert!(entry.clone().validate().is_ok());
    }

    #[test]
    fn test_entry_validate_success_command() {
        let entry = Entry::from_command_str("main.cpp", "clang -c", "/tmp", None);

        assert!(!entry.command.is_empty());
        assert!(entry.arguments.is_empty());

        assert!(entry.clone().validate().is_ok());
    }

    #[test]
    fn test_entry_validate_errors() {
        let cases = vec![
            (
                Entry::from_arguments_str("", vec!["clang", "-c"], "/tmp", None),
                EntryError::EmptyFileName,
            ),
            (
                Entry::from_arguments_str("main.cpp", vec!["clang", "-c"], "", None),
                EntryError::EmptyDirectory,
            ),
            (
                Entry {
                    file: "main.cpp".into(),
                    arguments: vec![],
                    command: "".to_string(),
                    directory: "/tmp".into(),
                    output: None,
                },
                EntryError::CommandOrArgumentsAreMissing,
            ),
            (
                Entry {
                    file: "main.cpp".into(),
                    arguments: vec!["clang".to_string()],
                    command: "clang".to_string(),
                    directory: "/tmp".into(),
                    output: None,
                },
                EntryError::CommandOrArgumentsArePresent,
            ),
            (
                Entry::from_command_str("main.cpp", "\"unterminated", "/tmp", None),
                EntryError::InvalidCommand(shell_words::ParseError),
            ),
        ];

        for (entry, expected_error) in cases {
            let err = entry.validate().unwrap_err();
            assert_eq!(err, expected_error);
        }
    }

    #[test]
    fn test_entry_with_arguments_constructor() {
        let entry = Entry::with_arguments(
            "main.cpp",
            vec!["clang".to_string(), "-c".to_string()],
            "/tmp",
            Some("main.o"),
        );

        assert!(!entry.arguments.is_empty());
        assert!(entry.command.is_empty());
        assert_eq!(entry.file, std::path::PathBuf::from("main.cpp"));
        assert_eq!(entry.directory, std::path::PathBuf::from("/tmp"));
        assert_eq!(entry.output, Some(std::path::PathBuf::from("main.o")));
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_entry_with_command_constructor() {
        let entry = Entry::with_command(
            "main.cpp",
            vec!["clang".to_string(), "-c".to_string()],
            "/tmp",
            Some("main.o"),
        );

        assert!(entry.arguments.is_empty());
        assert!(!entry.command.is_empty());
        assert_eq!(entry.command, "clang -c");
        assert_eq!(entry.file, std::path::PathBuf::from("main.cpp"));
        assert_eq!(entry.directory, std::path::PathBuf::from("/tmp"));
        assert_eq!(entry.output, Some(std::path::PathBuf::from("main.o")));
        assert!(entry.validate().is_ok());
    }
}

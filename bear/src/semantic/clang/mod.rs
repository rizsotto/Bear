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

mod converter;
mod filter;

use serde::{Deserialize, Serialize};
use shell_words;
use std::borrow::Cow;
use std::path;
use thiserror::Error;

// Re-export types for easier access
pub use converter::EntryConverter;
pub use filter::DuplicateEntryFilter;

/// Represents an entry of the compilation database.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The main translation unit source processed by this compilation step.
    /// This is used by tools as the key into the compilation database.
    /// There can be multiple command objects for the same file, for example if the same
    /// source file is compiled with different configurations.
    pub file: path::PathBuf,
    /// The compile command argv as list of strings. This should run the compilation step
    /// for the translation unit file. `arguments[0]` should be the executable name, such
    /// as `clang++`. Arguments should not be escaped, but ready to pass to `execvp()`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub arguments: Vec<String>,
    /// The compile command as a single shell-escaped string. Arguments may be shell quoted
    /// and escaped following platform conventions, with ‘"’ and ‘\’ being the only special
    /// characters. Shell expansion is not supported.
    ///
    /// Either `arguments` or `command` is required. `arguments` is preferred, as shell
    /// (un)escaping is a possible source of errors.
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub command: String,
    /// The working directory of the compilation. All paths specified in the `command` or
    /// `file` fields must be either absolute or relative to this directory.
    pub directory: path::PathBuf,
    /// The name of the output created by this compilation step. This field is optional.
    /// It can be used to distinguish different processing modes of the same input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub output: Option<path::PathBuf>,
}

impl Entry {
    /// Create an Entry from arguments (preferred).
    pub fn from_arguments(
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

    /// Convert entry to a form when only the command field is available.
    pub fn as_command(&self) -> Cow<Self> {
        if !self.command.is_empty() {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(Self {
                command: shell_words::join(&self.arguments),
                arguments: Vec::default(),
                ..self.clone()
            })
        }
    }

    /// Convert entry to a form when only the arguments field is available.
    ///
    /// The method can fail if the command field does not contain a valid shell escaped string.
    pub fn as_arguments(&self) -> Result<Cow<Self>, EntryError> {
        if !self.arguments.is_empty() {
            Ok(Cow::Borrowed(self))
        } else {
            let arguments = shell_words::split(&self.command)?;
            Ok(Cow::Owned(Self {
                arguments,
                command: String::default(),
                ..self.clone()
            }))
        }
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
    fn test_entry_conversions() {
        let entries = vec![
            Entry::from_arguments_str("main.cpp", vec!["clang", "-c", "main.cpp"], "/tmp", None),
            Entry::from_command_str("main.cpp", "clang -c main.cpp", "/tmp", None),
            Entry::from_arguments_str("foo.c", vec!["gcc", "-c", "foo.c"], "/src", Some("foo.o")),
            Entry::from_command_str("bar.c", "gcc -O2 -c bar.c", "/src", Some("bar.o")),
        ];

        for entry in entries {
            // arguments -> command -> arguments
            let to_cmd = entry.as_command();
            let to_args = to_cmd.as_arguments().unwrap();
            let to_cmd_again = to_args.as_command();
            assert_eq!(*to_cmd, *to_cmd_again);
            let to_args_again = to_cmd_again.as_arguments().unwrap();
            assert_eq!(*to_args, *to_args_again);
        }
    }

    #[test]
    fn test_cow_optimization() {
        // Test that as_command() returns borrowed when command field is already present
        let entry_with_command =
            Entry::from_command_str("main.cpp", "clang -c main.cpp", "/tmp", None);
        let cow_result = entry_with_command.as_command();
        assert!(matches!(cow_result, std::borrow::Cow::Borrowed(_)));

        // Test that as_arguments() returns borrowed when arguments field is already present
        let entry_with_args =
            Entry::from_arguments_str("main.cpp", vec!["clang", "-c", "main.cpp"], "/tmp", None);
        let cow_result = entry_with_args.as_arguments().unwrap();
        assert!(matches!(cow_result, std::borrow::Cow::Borrowed(_)));

        // Test that as_command() returns owned when conversion is needed
        let entry_with_args_only =
            Entry::from_arguments_str("main.cpp", vec!["clang", "-c", "main.cpp"], "/tmp", None);
        let cow_result = entry_with_args_only.as_command();
        assert!(matches!(cow_result, std::borrow::Cow::Owned(_)));

        // Test that as_arguments() returns owned when conversion is needed
        let entry_with_command_only =
            Entry::from_command_str("main.cpp", "clang -c main.cpp", "/tmp", None);
        let cow_result = entry_with_command_only.as_arguments().unwrap();
        assert!(matches!(cow_result, std::borrow::Cow::Owned(_)));
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! Testing utilities for the semantic analysis module.
//!
//! This module provides test-only helper functions and types for creating
//! and comparing semantic analysis structures in unit tests.

use super::{ArgumentKind, Arguments, CompilerCommand};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Simple test-only implementation of Arguments trait.
///
/// This is used in tests to create mock compiler arguments without depending
/// on production argument implementations.
#[derive(Debug, Clone, PartialEq)]
pub struct TestArguments {
    args: Vec<String>,
    kind: ArgumentKind,
}

impl TestArguments {
    pub fn new(args: Vec<String>, kind: ArgumentKind) -> Self {
        Self { args, kind }
    }
}

impl Arguments for TestArguments {
    fn kind(&self) -> ArgumentKind {
        self.kind.clone()
    }

    fn as_arguments(&self, _path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Vec<String> {
        self.args.clone()
    }

    fn as_file(&self, _path_updater: &dyn Fn(&Path) -> Cow<Path>) -> Option<PathBuf> {
        match self.kind {
            ArgumentKind::Source => self.args.first().map(PathBuf::from),
            ArgumentKind::Output => self.args.get(1).map(PathBuf::from),
            _ => None,
        }
    }
}

impl CompilerCommand {
    /// Create a CompilerCommand from string arguments for testing.
    ///
    /// This helper method creates a `CompilerCommand` with `TestArguments`
    /// for use in unit tests. Each argument group is created with the specified
    /// kind and string values.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cmd = CompilerCommand::from_strings(
    ///     "/home/user",
    ///     "/usr/bin/gcc",
    ///     vec![
    ///         (ArgumentKind::Source, vec!["main.c"]),
    ///         (ArgumentKind::Output, vec!["-o", "main.o"]),
    ///     ],
    /// );
    /// ```
    pub fn from_strings(
        working_dir: &str,
        executable: &str,
        arguments: Vec<(ArgumentKind, Vec<&str>)>,
    ) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            executable: PathBuf::from(executable),
            arguments: arguments
                .into_iter()
                .map(|(kind, args)| {
                    Box::new(TestArguments::new(
                        args.into_iter().map(String::from).collect(),
                        kind,
                    )) as Box<dyn Arguments>
                })
                .collect(),
        }
    }

    /// Compare two CompilerCommands by their arguments for testing.
    ///
    /// This method compares two compiler commands by checking if they have
    /// the same argument structure (kinds and values). Useful for test assertions
    /// and validation.
    ///
    /// # Returns
    ///
    /// `true` if both commands have identical arguments (same kinds and values
    /// in the same order), `false` otherwise.
    pub fn has_same_arguments(&self, other: &CompilerCommand) -> bool {
        if self.arguments.len() != other.arguments.len() {
            return false;
        }

        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);

        self.arguments
            .iter()
            .zip(other.arguments.iter())
            .all(|(a, b)| {
                a.kind() == b.kind()
                    && a.as_arguments(&path_updater) == b.as_arguments(&path_updater)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_command_comparison() {
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd3 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["other.c"]),
                (ArgumentKind::Output, vec!["-o", "other.o"]),
            ],
        );

        // Same arguments should be equal
        assert!(cmd1.has_same_arguments(&cmd2));

        // Different arguments should not be equal
        assert!(!cmd1.has_same_arguments(&cmd3));
    }

    #[test]
    fn test_arguments_with_different_kinds() {
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["main.c"])],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Output, vec!["main.c"])], // Same value, different kind
        );

        // Different argument kinds should not be equal
        assert!(!cmd1.has_same_arguments(&cmd2));
    }

    #[test]
    fn test_arguments_with_different_lengths() {
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source, vec!["main.c"])],
        );

        // Different number of arguments should not be equal
        assert!(!cmd1.has_same_arguments(&cmd2));
    }

    #[test]
    fn test_test_arguments_implementation() {
        let source_arg = TestArguments::new(vec!["main.c".to_string()], ArgumentKind::Source);
        let output_arg = TestArguments::new(
            vec!["-o".to_string(), "main.o".to_string()],
            ArgumentKind::Output,
        );
        let other_arg = TestArguments::new(vec!["-Wall".to_string()], ArgumentKind::Other(None));

        // Test kind method
        assert_eq!(source_arg.kind(), ArgumentKind::Source);
        assert_eq!(output_arg.kind(), ArgumentKind::Output);
        assert_eq!(other_arg.kind(), ArgumentKind::Other(None));

        // Test as_arguments method
        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        assert_eq!(source_arg.as_arguments(path_updater), vec!["main.c"]);
        assert_eq!(output_arg.as_arguments(path_updater), vec!["-o", "main.o"]);
        assert_eq!(other_arg.as_arguments(path_updater), vec!["-Wall"]);

        // Test as_file method
        assert_eq!(
            source_arg.as_file(path_updater),
            Some(PathBuf::from("main.c"))
        );
        assert_eq!(
            output_arg.as_file(path_updater),
            Some(PathBuf::from("main.o"))
        );
        assert_eq!(other_arg.as_file(path_updater), None);
    }
}

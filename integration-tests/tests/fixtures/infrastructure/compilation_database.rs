// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

/// Compilation database wrapper with assertion helpers
#[allow(dead_code)]
#[derive(Debug)]
pub struct CompilationDatabase {
    pub(super) entries: Vec<Value>,
}

impl CompilationDatabase {
    /// Assert the number of entries
    #[allow(dead_code)]
    pub fn assert_count(&self, expected: usize) -> Result<()> {
        let actual = self.entries.len();
        if actual != expected {
            anyhow::bail!("Expected {} compilation entries, but found {}", expected, actual);
        }
        Ok(())
    }

    /// Assert that the database contains an entry matching the criteria
    #[allow(dead_code)]
    pub fn assert_contains(&self, matcher: &CompilationEntryMatcher) -> Result<()> {
        let found = self.entries.iter().any(|entry| matcher.matches(entry));
        if !found {
            anyhow::bail!(
                "Expected to find compilation entry matching: {:?}\nActual entries: {:#?}",
                matcher,
                self.entries
            );
        }
        Ok(())
    }

    /// Get all entries
    #[allow(dead_code)]
    pub fn entries(&self) -> &[Value] {
        &self.entries
    }
}

/// Matcher for compilation database entries
#[derive(Debug)]
pub struct CompilationEntryMatcher {
    pub file: Option<String>,
    pub directory: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub output: Option<String>,
}

impl CompilationEntryMatcher {
    pub fn new() -> Self {
        Self { file: None, directory: None, arguments: None, output: None }
    }

    pub fn file<S: Into<String>>(mut self, file: S) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn directory<S: Into<String>>(mut self, directory: S) -> Self {
        self.directory = Some(directory.into());
        self
    }

    pub fn arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = Some(arguments);
        self
    }

    #[allow(dead_code)]
    pub fn output<S: Into<String>>(mut self, output: S) -> Self {
        self.output = Some(output.into());
        self
    }

    pub(super) fn matches(&self, entry: &Value) -> bool {
        if let Some(ref expected_file) = self.file {
            if let Some(actual_file) = entry.get("file").and_then(|v| v.as_str()) {
                if actual_file != expected_file {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref expected_dir) = self.directory {
            if let Some(actual_dir) = entry.get("directory").and_then(|v| v.as_str()) {
                // Canonicalize both paths for comparison on platforms with symlinks (e.g., macOS /var -> /private/var)
                let expected_canonical =
                    std::fs::canonicalize(expected_dir).unwrap_or_else(|_| PathBuf::from(expected_dir));
                let actual_canonical =
                    std::fs::canonicalize(actual_dir).unwrap_or_else(|_| PathBuf::from(actual_dir));

                if expected_canonical != actual_canonical {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(ref expected_args) = self.arguments {
            // Check both 'arguments' field (array) and 'command' field (string)
            let actual_args = if let Some(args_array) = entry.get("arguments").and_then(|v| v.as_array()) {
                args_array.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>()
            } else if let Some(command_str) = entry.get("command").and_then(|v| v.as_str()) {
                // Parse shell command string into arguments
                shell_words::split(command_str).unwrap_or_default()
            } else {
                return false;
            };

            if &actual_args != expected_args {
                return false;
            }
        }

        if let Some(ref expected_output) = self.output {
            if let Some(actual_output) = entry.get("output").and_then(|v| v.as_str()) {
                if actual_output != expected_output {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

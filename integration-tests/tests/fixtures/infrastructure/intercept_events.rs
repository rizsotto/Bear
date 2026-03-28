// SPDX-License-Identifier: GPL-3.0-or-later

use super::BearOutput;
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

/// Intercept events wrapper with assertion helpers
#[allow(dead_code)]
#[derive(Debug)]
pub struct InterceptEvents {
    pub(super) events: Vec<Value>,
    pub(super) verbose: bool,
    pub(super) bear_output: Option<BearOutput>,
}

impl InterceptEvents {
    /// Assert the number of events
    #[allow(dead_code)]
    pub fn assert_count(&self, expected: usize) -> Result<()> {
        let actual = self.events.len();
        if actual != expected {
            if self.verbose {
                // Show Bear command output first
                if let Some(ref bear_output) = self.bear_output {
                    eprintln!("\n=== Bear Command Output ===");
                    bear_output.show_verbose_output();
                    eprintln!("=== End Bear Output ===\n");
                }

                eprintln!("=== Events File Debug Info ===");
                eprintln!("Expected {} events, but found {}", expected, actual);
                eprintln!("Actual events:");
                for (i, event) in self.events.iter().enumerate() {
                    eprintln!(
                        "  Event {}: {}",
                        i,
                        serde_json::to_string_pretty(event).unwrap_or_else(|_| format!("{:?}", event))
                    );
                }
                eprintln!("=== End Debug Info ===\n");
            }
            anyhow::bail!("Expected {} intercept events, but found {}", expected, actual);
        }
        Ok(())
    }

    /// Assert that the events contain an entry matching the criteria
    #[allow(dead_code)]
    pub fn assert_contains(&self, matcher: &EventMatcher) -> Result<()> {
        let found = self.events.iter().any(|event| matcher.matches(event));
        if !found {
            if self.verbose {
                // Show Bear command output first
                if let Some(ref bear_output) = self.bear_output {
                    eprintln!("\n=== Bear Command Output ===");
                    bear_output.show_verbose_output();
                    eprintln!("=== End Bear Output ===\n");
                }

                eprintln!("=== Events File Debug Info ===");
                eprintln!("Failed to find event matching: {:?}", matcher);
                eprintln!("Actual events:");
                for (i, event) in self.events.iter().enumerate() {
                    eprintln!(
                        "  Event {}: {}",
                        i,
                        serde_json::to_string_pretty(event).unwrap_or_else(|_| format!("{:?}", event))
                    );
                }
                eprintln!("=== End Debug Info ===\n");
            }
            anyhow::bail!(
                "Expected to find intercept event matching: {:?}\nActual events: {:#?}",
                matcher,
                self.events
            );
        }
        Ok(())
    }

    /// Count events matching specific criteria
    #[allow(dead_code)]
    pub fn count_matching(&self, matcher: &EventMatcher) -> usize {
        self.events.iter().filter(|event| matcher.matches(event)).count()
    }

    /// Assert that events contain a specific number of entries matching the criteria
    #[allow(dead_code)]
    pub fn assert_count_matching(&self, matcher: &EventMatcher, expected: usize) -> Result<()> {
        let actual = self.count_matching(matcher);
        if actual != expected {
            if self.verbose {
                // Show Bear command output first
                if let Some(ref bear_output) = self.bear_output {
                    eprintln!("\n=== Bear Command Output ===");
                    bear_output.show_verbose_output();
                    eprintln!("=== End Bear Output ===\n");
                }

                eprintln!("=== Events File Debug Info ===");
                eprintln!("Expected {} events matching {:?}, but found {}", expected, matcher, actual);
                eprintln!("Matching events:");
                for (i, event) in self.events.iter().enumerate() {
                    if matcher.matches(event) {
                        eprintln!(
                            "  Event {}: {}",
                            i,
                            serde_json::to_string_pretty(event).unwrap_or_else(|_| format!("{:?}", event))
                        );
                    }
                }
                eprintln!("=== End Debug Info ===\n");
            }
            anyhow::bail!(
                "Expected {} intercept events matching {:?}, but found {}",
                expected,
                matcher,
                actual
            );
        }
        Ok(())
    }

    /// Assert that there are at least the minimum number of events
    #[allow(dead_code)]
    pub fn assert_min_count(&self, min_expected: usize) -> Result<()> {
        let actual = self.events.len();
        if actual < min_expected {
            if self.verbose {
                // Show Bear command output first
                if let Some(ref bear_output) = self.bear_output {
                    eprintln!("\n=== Bear Command Output ===");
                    bear_output.show_verbose_output();
                    eprintln!("=== End Bear Output ===\n");
                }

                eprintln!("=== Events File Debug Info ===");
                eprintln!("Expected at least {} events, but found {}", min_expected, actual);
                eprintln!("Actual events:");
                for (i, event) in self.events.iter().enumerate() {
                    eprintln!(
                        "  Event {}: {}",
                        i,
                        serde_json::to_string_pretty(event).unwrap_or_else(|_| format!("{:?}", event))
                    );
                }
                eprintln!("=== End Debug Info ===\n");
            }
            anyhow::bail!("Expected at least {} intercept events, but found {}", min_expected, actual);
        }
        Ok(())
    }

    /// Get all events
    #[allow(dead_code)]
    pub fn events(&self) -> &[Value] {
        &self.events
    }
}

/// Matcher for intercept events
#[derive(Debug)]
pub struct EventMatcher {
    pub executable_name: Option<String>,
    pub executable_path: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub working_directory: Option<String>,
    pub event_type: Option<String>,
}

impl EventMatcher {
    pub fn new() -> Self {
        Self {
            executable_name: None,
            executable_path: None,
            arguments: None,
            working_directory: None,
            event_type: None,
        }
    }

    pub fn executable_name<S: Into<String>>(mut self, name: S) -> Self {
        self.executable_name = Some(name.into());
        self
    }

    pub fn executable_path<S: Into<String>>(mut self, path: S) -> Self {
        self.executable_path = Some(path.into());
        self
    }

    pub fn arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = Some(arguments);
        self
    }

    #[allow(dead_code)]
    pub fn working_directory<S: Into<String>>(mut self, dir: S) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    #[allow(dead_code)]
    pub fn event_type<S: Into<String>>(mut self, event_type: S) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub(super) fn matches(&self, event: &Value) -> bool {
        // Check event type if specified
        if self.event_type.as_ref().is_some_and(|expected_type| event.get(expected_type).is_none()) {
            return false;
        }

        // Get the execution part of the event (most common case)
        let execution = match event.get("execution") {
            Some(exec) => exec,
            None => {
                // If no execution field and we're looking for execution-specific fields, fail
                if self.executable_name.is_some()
                    || self.executable_path.is_some()
                    || self.arguments.is_some()
                {
                    return false;
                }
                return true; // No execution field but we're not looking for execution-specific things
            }
        };

        // Check executable path
        if let Some(ref expected_path) = self.executable_path {
            if let Some(actual_path) = execution.get("executable").and_then(|v| v.as_str()) {
                if actual_path != expected_path {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check executable name (basename of executable path)
        if let Some(ref expected_name) = self.executable_name {
            if let Some(actual_path) = execution.get("executable").and_then(|v| v.as_str()) {
                let actual_name = std::path::Path::new(actual_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(actual_path);
                if actual_name != expected_name && !actual_name.contains(expected_name) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check arguments
        if let Some(ref expected_args) = self.arguments {
            if let Some(actual_args) = execution.get("arguments").and_then(|v| v.as_array()) {
                let actual_str_args: Vec<String> =
                    actual_args.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();
                if &actual_str_args != expected_args {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check working directory
        if let Some(ref expected_dir) = self.working_directory {
            if let Some(actual_dir) = execution.get("working_directory").and_then(|v| v.as_str()) {
                // Canonicalize both paths for comparison
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

        true
    }
}

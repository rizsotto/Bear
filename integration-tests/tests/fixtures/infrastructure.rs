// SPDX-License-Identifier: GPL-3.0-or-later

//! Test infrastructure for Bear integration tests
//!
//! This module provides utilities for setting up test environments,
//! running bear commands, and validating outputs. It's designed to
//! replicate the functionality of the Python/lit test suite.
//!
//! # Verbose Output Support
//!
//! The infrastructure supports verbose output display for debugging failed tests:
//!
//! ## Automatic Verbose Mode
//! Set `BEAR_TEST_VERBOSE=1` environment variable to automatically show bear output
//! when any test fails (panics).
//!
//! ## Manual Verbose Control
//! ```ignore
//! let env = TestEnvironment::new_with_verbose("test_name", true)?;
//! let output = env.run_bear(&["--output", "db.json", "--", "make"])?;
//!
//! // Show output only if verbose mode is enabled
//! output.show_verbose_if_enabled();
//!
//! // Force show output regardless of verbose setting
//! output.force_show_verbose();
//!
//! // Show last bear output from environment
//! env.show_last_bear_output();
//! ```
//!
//! ## Macro Support
//! ```ignore
//! bear_test!(my_test, verbose: true, |env| {
//!     let output = env.run_bear(&["semantic"])?;
//!     let db = env.load_compilation_database("compile_commands.json")?;
//!     db.assert_count(1)?; // Will show verbose output if this fails
//!     Ok(())
//! });
//! ```

use super::constants::*;
use anyhow::{Context, Result};
use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
// TempDir and predicates are infrastructure for future tests
#[allow(unused_imports)]
use assert_fs::{prelude::*, TempDir};
#[allow(unused_imports)]
use predicates::prelude::*;
use serde_json::{self, Value};

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use tempfile;

/// Test environment for Bear integration tests
///
/// Manages temporary directories, file setup, and cleanup with
/// debugging preservation on test failure.
#[derive(Debug)]
pub struct TestEnvironment {
    temp_dir: tempfile::TempDir,
    test_name: String,
    preserve_on_failure: bool,
    verbose: bool,
    last_bear_output: std::cell::RefCell<Option<BearOutput>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new(test_name: &str) -> Result<Self> {
        let temp_dir = tempfile::TempDir::new()
            .with_context(|| format!("Failed to create temp dir for test: {}", test_name))?;

        let preserve_on_failure = std::env::var("BEAR_TEST_PRESERVE_FAILURES")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let verbose = std::env::var("BEAR_TEST_VERBOSE")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            temp_dir,
            test_name: test_name.to_string(),
            preserve_on_failure,
            verbose,
            last_bear_output: std::cell::RefCell::new(None),
        })
    }

    /// Create a new test environment with explicit verbose setting
    #[allow(dead_code)]
    pub fn new_with_verbose(test_name: &str) -> Result<Self> {
        let temp_dir = tempfile::TempDir::new()
            .with_context(|| format!("Failed to create temp dir for test: {}", test_name))?;

        let preserve_on_failure = std::env::var("BEAR_TEST_PRESERVE_FAILURES")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            temp_dir,
            test_name: test_name.to_string(),
            preserve_on_failure,
            verbose: true,
            last_bear_output: std::cell::RefCell::new(None),
        })
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create source files in the test directory
    pub fn create_source_files(&self, files: &[(&str, &str)]) -> Result<()> {
        for (path, content) in files {
            let file_path = self.temp_dir().join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {:?}", parent))?;
            }
            fs::write(&file_path, content)
                .with_context(|| format!("Failed to write file: {}", path))?;
        }
        Ok(())
    }

    /// Create a build script in the test directory
    #[allow(dead_code)]
    pub fn create_build_script(&self, script_name: &str, content: &str) -> Result<PathBuf> {
        let script_path = self.temp_dir().join(script_name);
        fs::write(&script_path, content)?;

        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        Ok(script_path)
    }

    /// Create a shell script with proper shebang and shell detection
    #[allow(dead_code)]
    pub fn create_shell_script(&self, script_name: &str, commands: &str) -> Result<PathBuf> {
        let content = format!("#!{}\n{}", SHELL_PATH, commands);
        self.create_build_script(script_name, &content)
    }

    /// Create a Makefile in the test directory
    #[allow(dead_code)]
    pub fn create_makefile(&self, makefile_name: &str, content: &str) -> Result<PathBuf> {
        let makefile_path = self.temp_dir().join(makefile_name);
        fs::write(&makefile_path, content)?;
        Ok(makefile_path)
    }

    /// Create a configuration file (YAML format)
    #[allow(dead_code)]
    pub fn create_config(&self, config_yaml: &str) -> Result<PathBuf> {
        let config_path = self.temp_dir().join("config.yml");
        fs::write(&config_path, config_yaml)?;
        Ok(config_path)
    }

    /// Run bear with the given arguments
    pub fn run_bear(&self, args: &[&str]) -> Result<BearOutput> {
        let mut cmd = Command::new(cargo_bin(BEAR_BIN));
        cmd.current_dir(self.temp_dir())
            .env("RUST_LOG", "debug")
            .env("RUST_BACKTRACE", "1")
            .args(args);

        let output = cmd.output()?;

        let bear_output = BearOutput {
            output,
            temp_dir: self.temp_dir().to_path_buf(),
            verbose: self.verbose,
        };

        // Store the output for potential later display
        *self.last_bear_output.borrow_mut() = Some(bear_output.clone());

        Ok(bear_output)
    }

    /// Run bear and expect success
    #[allow(dead_code)]
    pub fn run_bear_success(&self, args: &[&str]) -> Result<BearOutput> {
        let result = self.run_bear(args)?;
        result.assert_success()?;
        Ok(result)
    }

    /// Run bear and expect failure
    #[allow(dead_code)]
    pub fn run_bear_failure(&self, args: &[&str]) -> Result<BearOutput> {
        let result = self.run_bear(args)?;
        result.assert_failure()?;
        Ok(result)
    }

    /// Check if a file exists in the test directory
    #[allow(dead_code)]
    pub fn file_exists(&self, path: &str) -> bool {
        self.temp_dir().join(path).exists()
    }

    /// Read file content from test directory
    #[allow(dead_code)]
    pub fn read_file(&self, path: &str) -> Result<String> {
        let file_path = self.temp_dir().join(path);
        fs::read_to_string(&file_path).with_context(|| format!("Failed to read file: {}", path))
    }

    /// Load compilation database from file
    #[allow(dead_code)]
    pub fn load_compilation_database(&self, path: &str) -> Result<CompilationDatabase> {
        let db_path = self.temp_dir().join(path);
        let content = fs::read_to_string(&db_path)
            .with_context(|| format!("Failed to read compilation database: {:?}", db_path))?;

        let entries: Vec<Value> = serde_json::from_str(&content)
            .with_context(|| "Failed to parse compilation database JSON")?;

        Ok(CompilationDatabase {
            entries,
            verbose: self.verbose,
            bear_output: self.last_bear_output.borrow().clone(),
        })
    }

    /// Show the last bear output for debugging
    pub fn show_last_bear_output(&self) {
        if let Some(ref output) = *self.last_bear_output.borrow() {
            output.show_verbose_output();
        } else {
            eprintln!("No bear output available to show");
        }
    }

    /// Get verbose mode setting
    #[allow(dead_code)]
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Preserve test directory for debugging if test fails
    fn preserve_on_panic(&self) {
        if self.preserve_on_failure && std::thread::panicking() {
            let preserve_dir = format!("/tmp/bear-test-{}-{}", self.test_name, std::process::id());

            if let Err(e) = fs::rename(self.temp_dir(), &preserve_dir) {
                eprintln!("Failed to preserve test directory: {}", e);
            } else {
                eprintln!("Test failed. Directory preserved at: {}", preserve_dir);
            }
        }

        // Show verbose output if enabled and test is failing
        if self.verbose && std::thread::panicking() {
            eprintln!("\n=== Bear Verbose Output (Test: {}) ===", self.test_name);
            self.show_last_bear_output();
            eprintln!("=== End Bear Output ===\n");
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        self.preserve_on_panic();
    }
}

/// Bear command output wrapper
#[allow(dead_code)]
#[derive(Debug)]
pub struct BearOutput {
    output: Output,
    temp_dir: PathBuf,
    verbose: bool,
}

impl Clone for BearOutput {
    fn clone(&self) -> Self {
        Self {
            output: Output {
                status: self.output.status,
                stdout: self.output.stdout.clone(),
                stderr: self.output.stderr.clone(),
            },
            temp_dir: self.temp_dir.clone(),
            verbose: self.verbose,
        }
    }
}

impl BearOutput {
    /// Show verbose output for debugging
    pub fn show_verbose_output(&self) {
        let stdout = String::from_utf8_lossy(&self.output.stdout);
        let stderr = String::from_utf8_lossy(&self.output.stderr);

        eprintln!("Bear stdout:");
        if stdout.is_empty() {
            eprintln!("  (empty)");
        } else {
            for line in stdout.lines() {
                eprintln!("  {}", line);
            }
        }

        eprintln!("Bear stderr:");
        if stderr.is_empty() {
            eprintln!("  (empty)");
        } else {
            for line in stderr.lines() {
                eprintln!("  {}", line);
            }
        }

        eprintln!("Bear exit code: {:?}", self.output.status.code());
    }

    /// Assert that bear command succeeded
    pub fn assert_success(&self) -> Result<()> {
        if !self.output.status.success() {
            let stderr = String::from_utf8_lossy(&self.output.stderr);
            let stdout = String::from_utf8_lossy(&self.output.stdout);
            anyhow::bail!(
                "Bear command failed with exit code: {:?}\nstdout: {}\nstderr: {}",
                self.output.status.code(),
                stdout,
                stderr
            );
        }
        Ok(())
    }

    /// Assert that bear command failed
    pub fn assert_failure(&self) -> Result<()> {
        if self.output.status.success() {
            anyhow::bail!("Expected bear command to fail, but it succeeded");
        }
        Ok(())
    }

    /// Get stdout as string
    #[allow(dead_code)]
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    /// Get stderr as string
    #[allow(dead_code)]
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// Get exit code
    #[allow(dead_code)]
    pub fn exit_code(&self) -> Option<i32> {
        self.output.status.code()
    }

    /// Show verbose output if verbose mode is enabled
    #[allow(dead_code)]
    pub fn show_verbose_if_enabled(&self) {
        if self.verbose {
            self.show_verbose_output();
        }
    }

    /// Force show verbose output regardless of verbose mode setting
    #[allow(dead_code)]
    pub fn force_show_verbose(&self) {
        eprintln!("\n=== Bear Command Output ===");
        self.show_verbose_output();
        eprintln!("=== End Bear Output ===\n");
    }
}

/// Compilation database wrapper with assertion helpers
#[allow(dead_code)]
#[derive(Debug)]
pub struct CompilationDatabase {
    entries: Vec<Value>,
    verbose: bool,
    bear_output: Option<BearOutput>,
}

impl CompilationDatabase {
    /// Assert the number of entries
    #[allow(dead_code)]
    pub fn assert_count(&self, expected: usize) -> Result<()> {
        let actual = self.entries.len();
        if actual != expected {
            if self.verbose {
                // Show Bear command output first
                if let Some(ref bear_output) = self.bear_output {
                    eprintln!("\n=== Bear Command Output ===");
                    bear_output.show_verbose_output();
                    eprintln!("=== End Bear Output ===\n");
                }

                eprintln!("=== Compilation Database Debug Info ===");
                eprintln!("Expected {} entries, but found {}", expected, actual);
                eprintln!("Actual entries:");
                for (i, entry) in self.entries.iter().enumerate() {
                    eprintln!(
                        "  Entry {}: {}",
                        i,
                        serde_json::to_string_pretty(entry)
                            .unwrap_or_else(|_| format!("{:?}", entry))
                    );
                }
                eprintln!("=== End Debug Info ===\n");
            }
            anyhow::bail!(
                "Expected {} compilation entries, but found {}",
                expected,
                actual
            );
        }
        Ok(())
    }

    /// Assert that the database contains an entry matching the criteria
    #[allow(dead_code)]
    pub fn assert_contains(&self, matcher: &CompilationEntryMatcher) -> Result<()> {
        let found = self.entries.iter().any(|entry| matcher.matches(entry));
        if !found {
            if self.verbose {
                // Show Bear command output first
                if let Some(ref bear_output) = self.bear_output {
                    eprintln!("\n=== Bear Command Output ===");
                    bear_output.show_verbose_output();
                    eprintln!("=== End Bear Output ===\n");
                }

                eprintln!("=== Compilation Database Debug Info ===");
                eprintln!("Failed to find entry matching: {:?}", matcher);
                eprintln!("Actual entries:");
                for (i, entry) in self.entries.iter().enumerate() {
                    eprintln!(
                        "  Entry {}: {}",
                        i,
                        serde_json::to_string_pretty(entry)
                            .unwrap_or_else(|_| format!("{:?}", entry))
                    );
                }
                eprintln!("=== End Debug Info ===\n");
            }
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
        Self {
            file: None,
            directory: None,
            arguments: None,
            output: None,
        }
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

    fn matches(&self, entry: &Value) -> bool {
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
                let expected_canonical = std::fs::canonicalize(expected_dir)
                    .unwrap_or_else(|_| PathBuf::from(expected_dir));
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
            let actual_args =
                if let Some(args_array) = entry.get("arguments").and_then(|v| v.as_array()) {
                    args_array
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
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

/// Helper macros for common test patterns
#[macro_export]
macro_rules! bear_test {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            let env = TestEnvironment::new(stringify!($name))?;
            $body(&env)
        }
    };
    ($name:ident, verbose: $verbose:expr, $body:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            let env = TestEnvironment::new_with_verbose(stringify!($name), $verbose)?;
            $body(&env)
        }
    };
}

#[macro_export]
macro_rules! compilation_entry {
    (file: $file:expr, directory: $dir:expr, arguments: $args:expr) => {
        $crate::fixtures::infrastructure::CompilationEntryMatcher::new()
            .file($file)
            .directory($dir)
            .arguments($args)
    };
}

// Re-export the macro at module level for easier importing
#[allow(unused_imports)]
pub use compilation_entry;

// Test helper functions for common operations
#[allow(dead_code)]
pub fn touch_file(env: &TestEnvironment, path: &str) -> Result<()> {
    env.create_source_files(&[(path, "")])?;
    Ok(())
}

#[allow(dead_code)]
pub fn create_c_file(env: &TestEnvironment, path: &str, content: &str) -> Result<()> {
    env.create_source_files(&[(path, content)])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_creation() -> Result<()> {
        let env = TestEnvironment::new("test_creation")?;
        assert!(env.temp_dir().exists());
        Ok(())
    }

    #[test]
    fn file_creation() -> Result<()> {
        let env = TestEnvironment::new("test_files")?;
        env.create_source_files(&[
            ("test.c", "int main() { return 0; }"),
            ("subdir/test.h", "#pragma once"),
        ])?;

        assert!(env.temp_dir().join("test.c").exists());
        assert!(env.temp_dir().join("subdir/test.h").exists());
        Ok(())
    }

    #[test]
    fn compilation_matcher() {
        let entry = serde_json::json!({
            "file": "/path/to/test.c",
            "directory": "/path/to",
            "arguments": ["gcc", "-c", "test.c"]
        });

        let matcher = CompilationEntryMatcher::new()
            .file("/path/to/test.c")
            .directory("/path/to")
            .arguments(vec![
                "gcc".to_string(),
                "-c".to_string(),
                "test.c".to_string(),
            ]);

        assert!(matcher.matches(&entry));
    }
}

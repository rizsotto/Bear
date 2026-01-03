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
use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
// TempDir and predicates are infrastructure for future tests
#[allow(unused_imports)]
use assert_fs::{TempDir, prelude::*};
use encoding_rs;
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
            fs::write(&file_path, content).with_context(|| format!("Failed to write file: {}", path))?;
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
    #[cfg(has_executable_shell)]
    pub fn create_shell_script(&self, script_name: &str, commands: &str) -> Result<PathBuf> {
        let content = format!("#!{}\n{}", SHELL_PATH, commands);
        self.create_build_script(script_name, &content)
    }

    /// Create shell script with specific encoding
    #[allow(dead_code)]
    #[cfg(has_executable_shell)]
    pub fn create_shell_script_with_encoding(
        &self,
        script_name: &str,
        commands: &str,
        encoding: &'static encoding_rs::Encoding,
    ) -> Result<PathBuf> {
        let content = format!("#!{}\n{}", SHELL_PATH, commands);
        self.create_build_script_with_encoding(script_name, &content, encoding)
    }

    /// Create build script with specific encoding
    pub fn create_build_script_with_encoding(
        &self,
        script_name: &str,
        content: &str,
        encoding: &'static encoding_rs::Encoding,
    ) -> Result<PathBuf> {
        let script_path = self.temp_dir().join(script_name);

        // Encode content using the specified encoding
        let (encoded_bytes, _, had_errors) = encoding.encode(content);
        if had_errors {
            return Err(anyhow::anyhow!("Failed to encode content with {:?}", encoding.name()));
        }

        // Write the encoded bytes to file
        fs::write(&script_path, &*encoded_bytes)?;

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

    /// Verify that a file is encoded with the specified encoding
    pub fn verify_file_encoding(
        &self,
        file_path: &Path,
        expected_encoding: &'static encoding_rs::Encoding,
    ) -> Result<bool> {
        let bytes = fs::read(file_path)?;

        // Try to decode with the expected encoding
        let (decoded_string, encoding_used, had_errors) = expected_encoding.decode(&bytes);

        // Check if decoding was successful and used the expected encoding
        Ok(!had_errors && encoding_used == expected_encoding && !decoded_string.is_empty())
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
        cmd.current_dir(self.temp_dir()).env("RUST_LOG", "debug").env("RUST_BACKTRACE", "1").args(args);

        let output = cmd.output()?;

        let bear_output =
            BearOutput { output, temp_dir: self.temp_dir().to_path_buf(), verbose: self.verbose };

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

    /// Run C compiler directly (without Bear) to compile test programs
    ///
    /// This method provides semantic separation between compilation and Bear interception.
    /// Use this for compiling test programs that will later be executed under Bear's
    /// intercept mode, rather than using `run_bear` which would run the compiler through Bear.
    ///
    /// # Arguments
    /// * `output_name` - Name of the executable to produce
    /// * `source_files` - Array of source file paths to compile
    ///
    /// # Example
    /// ```ignore
    /// env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
    /// let executable_path = env.run_c_compiler("test_program", &["test.c"])?;
    /// env.run_bear(&["intercept", "--output", "events.json", "--", "./test_program"])?;
    /// ```
    #[allow(dead_code)]
    #[cfg(has_executable_compiler_c)]
    pub fn run_c_compiler(&self, output_name: &str, source_files: &[&str]) -> Result<PathBuf> {
        let mut cmd = std::process::Command::new(COMPILER_C_PATH);
        cmd.current_dir(self.temp_dir());

        // Add output flag
        cmd.arg("-o").arg(output_name);

        // Add source files
        for source in source_files {
            cmd.arg(source);
        }

        let output =
            cmd.output().with_context(|| format!("Failed to run C compiler: {}", COMPILER_C_PATH))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "C compiler failed with exit code {:?}:\nstdout: {}\nstderr: {}",
                output.status.code(),
                stdout,
                stderr
            );
        }

        // On Windows, executables have .exe extension
        let executable_name =
            if cfg!(windows) { format!("{}.exe", output_name) } else { output_name.to_string() };

        let executable_path = self.temp_dir().join(executable_name);
        Ok(executable_path)
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

        let entries: Vec<Value> =
            serde_json::from_str(&content).with_context(|| "Failed to parse compilation database JSON")?;

        Ok(CompilationDatabase {
            entries,
            verbose: self.verbose,
            bear_output: self.last_bear_output.borrow().clone(),
        })
    }

    /// Load intercept events file
    #[allow(dead_code)]
    pub fn load_events_file(&self, path: &str) -> Result<InterceptEvents> {
        let events_path = self.temp_dir().join(path);
        let content = fs::read_to_string(&events_path)
            .with_context(|| format!("Failed to read events file: {:?}", events_path))?;

        let events: Vec<Value> = content
            .lines()
            .map(|line| serde_json::from_str(line))
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| "Failed to parse events JSON lines")?;

        Ok(InterceptEvents {
            events,
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
                        serde_json::to_string_pretty(entry).unwrap_or_else(|_| format!("{:?}", entry))
                    );
                }
                eprintln!("=== End Debug Info ===\n");
            }
            anyhow::bail!("Expected {} compilation entries, but found {}", expected, actual);
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
                        serde_json::to_string_pretty(entry).unwrap_or_else(|_| format!("{:?}", entry))
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

/// Intercept events wrapper with assertion helpers
#[allow(dead_code)]
#[derive(Debug)]
pub struct InterceptEvents {
    events: Vec<Value>,
    verbose: bool,
    bear_output: Option<BearOutput>,
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

    fn matches(&self, event: &Value) -> bool {
        // Check event type if specified
        if let Some(ref expected_type) = self.event_type {
            if !event.get(expected_type).is_some() {
                return false;
            }
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

#[macro_export]
macro_rules! event_matcher {
    (executable_path: $path:expr) => {
        $crate::fixtures::infrastructure::EventMatcher::new().executable_path($path)
    };
    (executable_name: $name:expr) => {
        $crate::fixtures::infrastructure::EventMatcher::new().executable_name($name)
    };
    (executable_path: $path:expr, arguments: $args:expr) => {
        $crate::fixtures::infrastructure::EventMatcher::new().executable_path($path).arguments($args)
    };
}

// Re-export the macros at module level for easier importing
#[allow(unused_imports)]
pub use compilation_entry;
#[allow(unused_imports)]
pub use event_matcher;

/// Helper function to get the appropriate compiler command for build scripts
/// Always uses just the filename to ensure compatibility across all platforms
pub fn filename_of(compiler_path: &str) -> String {
    Path::new(compiler_path).file_name().unwrap().to_string_lossy().to_string()
}

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
            .arguments(vec!["gcc".to_string(), "-c".to_string(), "test.c".to_string()]);

        assert!(matcher.matches(&entry));
    }

    #[test]
    fn event_matcher_executable_path() {
        let event = serde_json::json!({
            "execution": {
                "executable": "/usr/bin/gcc",
                "arguments": ["gcc", "-c", "test.c"],
                "working_directory": "/tmp"
            }
        });

        let matcher = EventMatcher::new().executable_path("/usr/bin/gcc");

        assert!(matcher.matches(&event));

        // Test non-matching path
        let matcher_no_match = EventMatcher::new().executable_path("/usr/bin/clang");

        assert!(!matcher_no_match.matches(&event));
    }

    #[test]
    fn event_matcher_executable_name() {
        let event = serde_json::json!({
            "execution": {
                "executable": "/usr/bin/gcc",
                "arguments": ["gcc", "-c", "test.c"]
            }
        });

        let matcher = EventMatcher::new().executable_name("gcc");

        assert!(matcher.matches(&event));

        // Test partial name matching
        let matcher_partial = EventMatcher::new().executable_name("gc");

        assert!(matcher_partial.matches(&event));

        // Test non-matching name
        let matcher_no_match = EventMatcher::new().executable_name("clang");

        assert!(!matcher_no_match.matches(&event));
    }

    #[test]
    fn event_matcher_arguments() {
        let event = serde_json::json!({
            "execution": {
                "executable": "/usr/bin/gcc",
                "arguments": ["gcc", "-c", "test.c", "-o", "test.o"]
            }
        });

        let matcher = EventMatcher::new().arguments(vec![
            "gcc".to_string(),
            "-c".to_string(),
            "test.c".to_string(),
            "-o".to_string(),
            "test.o".to_string(),
        ]);

        assert!(matcher.matches(&event));

        // Test non-matching arguments
        let matcher_no_match =
            EventMatcher::new().arguments(vec!["gcc".to_string(), "-c".to_string(), "other.c".to_string()]);

        assert!(!matcher_no_match.matches(&event));
    }

    #[test]
    fn event_matcher_no_execution() {
        let event = serde_json::json!({
            "other_field": "value"
        });

        let matcher = EventMatcher::new().executable_path("/usr/bin/gcc");

        // Should not match if looking for execution fields but no execution present
        assert!(!matcher.matches(&event));

        // Should match if not looking for execution-specific fields
        let empty_matcher = EventMatcher::new();
        assert!(empty_matcher.matches(&event));
    }

    #[test]
    #[cfg(has_executable_compiler_c)]
    fn run_c_compiler_basic() -> Result<()> {
        let env = TestEnvironment::new("test_c_compiler")?;

        // Create a simple C program
        env.create_source_files(&[(
            "hello.c",
            r#"
#include <stdio.h>
int main() {
    printf("Hello, World!\n");
    return 0;
}
"#,
        )])?;

        // Compile it using our new method
        let executable_path = env.run_c_compiler("hello", &["hello.c"])?;

        // Verify the executable exists at the returned path
        assert!(executable_path.exists());

        // Verify the executable actually works by running it
        let output = std::process::Command::new(&executable_path)
            .current_dir(env.temp_dir())
            .output()
            .expect("Failed to run compiled executable");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello, World!"));

        Ok(())
    }

    #[test]
    #[cfg(has_executable_compiler_c)]
    fn run_c_compiler_error_handling() -> Result<()> {
        let env = TestEnvironment::new("test_c_compiler_error")?;

        // Create a C program with syntax errors
        env.create_source_files(&[(
            "broken.c",
            r#"
#include <stdio.h>
int main() {
    printf("Hello, World!\n"  // Missing closing parenthesis and semicolon
    return 0;
}
"#,
        )])?;

        // Compilation should fail and return an error
        let compile_result = env.run_c_compiler("broken", &["broken.c"]);
        assert!(compile_result.is_err());

        Ok(())
    }

    #[test]
    fn assert_min_count_test() -> Result<()> {
        use serde_json::json;

        // Create mock events
        let events = vec![
            json!({"execution": {"executable": "/usr/bin/gcc", "arguments": ["gcc", "-c", "test.c"]}}),
            json!({"execution": {"executable": "/usr/bin/gcc", "arguments": ["gcc", "-c", "test2.c"]}}),
        ];

        let intercept_events = InterceptEvents { events, verbose: false, bear_output: None };

        // Should pass when actual >= min_expected
        assert!(intercept_events.assert_min_count(1).is_ok());
        assert!(intercept_events.assert_min_count(2).is_ok());

        // Should fail when actual < min_expected
        assert!(intercept_events.assert_min_count(3).is_err());

        Ok(())
    }
}

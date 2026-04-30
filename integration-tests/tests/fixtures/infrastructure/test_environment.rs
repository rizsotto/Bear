// SPDX-License-Identifier: GPL-3.0-or-later

use super::{BearOutput, CompilationDatabase, InstallEnvironment, InterceptEvents};
use crate::fixtures::constants::*;
use anyhow::{Context, Result};
use assert_cmd::Command;
#[allow(unused_imports)]
use assert_fs::{TempDir, prelude::*};
#[allow(unused_imports)]
use predicates::prelude::*;
use serde_json::{self, Value};

use std::fs;
use std::path::{Path, PathBuf};

/// Test environment for Bear integration tests
///
/// Manages temporary directories, file setup, and cleanup with
/// debugging preservation on test failure.
pub struct TestEnvironment {
    install: InstallEnvironment,
    test_dir: tempfile::TempDir,
    test_name: String,
    preserve_on_failure: bool,
    last_bear_output: std::cell::RefCell<Option<BearOutput>>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub fn new(test_name: &str) -> Result<Self> {
        let install = InstallEnvironment::new()?;

        let test_dir = tempfile::TempDir::new()
            .with_context(|| format!("Failed to create temp dir for test: {}", test_name))?;

        let preserve_on_failure = std::env::var("BEAR_TEST_PRESERVE_FAILURES")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Self {
            install,
            test_dir,
            test_name: test_name.to_string(),
            preserve_on_failure,
            last_bear_output: std::cell::RefCell::new(None),
        })
    }

    /// Get the temporary directory path
    pub fn test_dir(&self) -> &Path {
        self.test_dir.path()
    }

    /// Create source files in the test directory
    pub fn create_source_files(&self, files: &[(&str, &str)]) -> Result<()> {
        for (path, content) in files {
            let file_path = self.test_dir().join(path);
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
        let script_path = self.test_dir().join(script_name);
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
        let script_path = self.test_dir().join(script_name);

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
    #[allow(dead_code)]
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
        let makefile_path = self.test_dir().join(makefile_name);
        fs::write(&makefile_path, content)?;
        Ok(makefile_path)
    }

    /// Create a configuration file (YAML format)
    #[allow(dead_code)]
    pub fn create_config(&self, config_yaml: &str) -> Result<PathBuf> {
        let config_path = self.test_dir().join("config.yml");
        fs::write(&config_path, config_yaml)?;
        Ok(config_path)
    }

    #[allow(dead_code)]
    pub fn command_bear(&self) -> std::process::Command {
        std::process::Command::new(self.install.path())
    }

    /// Run bear with the given arguments.
    ///
    /// `RUST_LOG` is inherited when set in the test process; otherwise it
    /// defaults to `info`. The default keeps warn/info/error log lines in
    /// captured stderr (so tests that assert on them work) without pulling
    /// in the noisy per-event `debug` traces from the preload library. CI
    /// sets `RUST_LOG=debug` explicitly to get the full diagnostic stream.
    /// `RUST_BACKTRACE=1` is always forced so panics in bear surface
    /// readable backtraces.
    pub fn run_bear(&self, args: &[&str]) -> Result<BearOutput> {
        let mut cmd = Command::new(self.install.path());
        cmd.current_dir(self.test_dir()).env("RUST_BACKTRACE", "1").args(args);
        if std::env::var_os("RUST_LOG").is_none() {
            cmd.env("RUST_LOG", "info");
        }

        let output = cmd.output()?;

        let bear_output = BearOutput { output, temp_dir: self.test_dir().to_path_buf() };

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
        cmd.current_dir(self.test_dir());

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

        let executable_path = self.test_dir().join(executable_name);
        Ok(executable_path)
    }

    /// Check if a file exists in the test directory
    #[allow(dead_code)]
    pub fn file_exists(&self, path: &str) -> bool {
        self.test_dir().join(path).exists()
    }

    /// Read file content from test directory
    #[allow(dead_code)]
    pub fn read_file(&self, path: &str) -> Result<String> {
        let file_path = self.test_dir().join(path);
        fs::read_to_string(&file_path).with_context(|| format!("Failed to read file: {}", path))
    }

    /// Load compilation database from file
    #[allow(dead_code)]
    pub fn load_compilation_database(&self, path: &str) -> Result<CompilationDatabase> {
        let db_path = self.test_dir().join(path);
        let content = fs::read_to_string(&db_path)
            .with_context(|| format!("Failed to read compilation database: {:?}", db_path))?;

        let entries: Vec<Value> =
            serde_json::from_str(&content).with_context(|| "Failed to parse compilation database JSON")?;

        Ok(CompilationDatabase { entries })
    }

    /// Load intercept events file
    #[allow(dead_code)]
    pub fn load_events_file(&self, path: &str) -> Result<InterceptEvents> {
        let events_path = self.test_dir().join(path);
        let content = fs::read_to_string(&events_path)
            .with_context(|| format!("Failed to read events file: {:?}", events_path))?;

        let events: Vec<Value> = content
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| "Failed to parse events JSON lines")?;

        Ok(InterceptEvents { events })
    }

    /// Dump the last captured bear output unconditionally; cargo's per-test
    /// capture discards it for passing tests and surfaces it for failing
    /// ones (both `Err` returns and panics). Optionally preserve the temp
    /// dir, but only on actual panic — `BEAR_TEST_PRESERVE_FAILURES` has
    /// always been panic-gated.
    fn preserve_on_panic(&self) {
        if let Some(ref output) = *self.last_bear_output.borrow() {
            eprintln!("\n=== Bear output (test: {}) ===", self.test_name);
            output.show_output();
            eprintln!("=== end ===\n");
        }

        if self.preserve_on_failure && std::thread::panicking() {
            let preserve_dir = format!("/tmp/bear-test-{}-{}", self.test_name, std::process::id());
            if let Err(e) = fs::rename(self.test_dir(), &preserve_dir) {
                eprintln!("Failed to preserve test directory: {}", e);
            } else {
                eprintln!("Test failed. Directory preserved at: {}", preserve_dir);
            }
        }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        self.preserve_on_panic();
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use std::path::PathBuf;
use std::process::Output;

/// Bear command output wrapper
#[allow(dead_code)]
#[derive(Debug)]
pub struct BearOutput {
    pub(super) output: Output,
    pub(super) temp_dir: PathBuf,
    pub(super) verbose: bool,
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

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
        }
    }
}

impl BearOutput {
    /// Dump bear's stdout, stderr, and exit code to the test binary's stderr.
    /// Used by `TestEnvironment::preserve_on_panic` for failure diagnostics.
    pub fn show_output(&self) {
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
}

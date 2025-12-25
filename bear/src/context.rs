// SPDX-License-Identifier: GPL-3.0-or-later

use crate::environment;
use anyhow::{Context as AnyhowContext, Result};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::PathBuf;

/// Application context containing runtime environment information.
///
/// This struct captures all the environmental context needed by Bear at startup,
/// including system information, environment variables, and file system state.
/// This allows for pure validation and configuration phases without additional
/// I/O operations.
#[derive(Debug, Clone)]
pub struct Context {
    /// Path to the current Bear executable
    pub current_executable: PathBuf,
    /// Current working directory when Bear was invoked
    pub current_directory: PathBuf,
    /// All environment variables at startup
    pub environment: HashMap<String, String>,
}

impl Context {
    /// Capture the current application context.
    ///
    /// This function performs I/O operations to gather system state and should
    /// be called early in the application lifecycle, before any validation phase.
    pub fn capture() -> Result<Self> {
        let current_executable =
            env::current_exe().with_context(|| "Failed to get current executable path")?;

        let current_directory =
            env::current_dir().with_context(|| "Failed to get current working directory")?;

        let environment = env::vars().collect::<HashMap<String, String>>();

        Ok(Context {
            current_executable,
            current_directory,
            environment,
        })
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Application Context:")?;
        writeln!(
            f,
            "  Current Executable: {}",
            self.current_executable.display()
        )?;
        writeln!(
            f,
            "  Current Directory: {}",
            self.current_directory.display()
        )?;
        writeln!(
            f,
            "  Total Environment Variables: {} entries",
            self.environment.len()
        )?;

        // Display relevant environment variables by iterating directly
        writeln!(f, "  Relevant Environment Variables:")?;
        for (key, value) in &self.environment {
            if environment::relevant_env(key) {
                writeln!(f, "    {}={}", key, value)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_capture() {
        let context = Context::capture();
        assert!(context.is_ok());

        let ctx = context.unwrap();

        // Basic assertions that should always be true
        assert!(ctx.current_directory.is_absolute());
        assert!(ctx.current_executable.is_absolute());
    }

    #[test]
    fn test_display_format() {
        let context = Context::capture().unwrap();
        let display_output = format!("{}", context);

        assert!(display_output.contains("Application Context:"));
        assert!(display_output.contains("Current Directory:"));
        assert!(display_output.contains("Current Executable:"));
        assert!(display_output.contains("Relevant Environment Variables:"));
        assert!(display_output.contains("Total Environment Variables:"));
    }

    #[test]
    fn test_display_includes_relevant_env_vars() {
        use std::collections::HashMap;

        // Create a test context with known environment variables
        let mut test_env = HashMap::new();
        test_env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        test_env.insert("CC".to_string(), "gcc".to_string());
        test_env.insert("IRRELEVANT_VAR".to_string(), "value".to_string());
        test_env.insert("CFLAGS".to_string(), "-O2".to_string());

        let context = Context {
            current_executable: std::env::current_exe().unwrap(),
            current_directory: std::env::current_dir().unwrap(),
            environment: test_env,
        };

        let display_output = format!("{}", context);

        // Should include relevant variables
        assert!(display_output.contains("PATH=/usr/bin:/bin"));
        assert!(display_output.contains("CC=gcc"));
        assert!(display_output.contains("CFLAGS=-O2"));

        // Should not include irrelevant variables
        assert!(!display_output.contains("IRRELEVANT_VAR=value"));

        // Should show relevant env vars section
        assert!(display_output.contains("Relevant Environment Variables:"));
        assert!(display_output.contains("Total Environment Variables: 4 entries"));
    }

    #[test]
    fn test_display_no_relevant_env_vars() {
        use std::collections::HashMap;

        // Create a test context with no relevant environment variables
        let mut test_env = HashMap::new();
        test_env.insert("IRRELEVANT_VAR1".to_string(), "value1".to_string());
        test_env.insert("IRRELEVANT_VAR2".to_string(), "value2".to_string());

        let context = Context {
            current_executable: std::env::current_exe().unwrap(),
            current_directory: std::env::current_dir().unwrap(),
            environment: test_env,
        };

        let display_output = format!("{}", context);

        // Should show that there are no relevant variables
        assert!(display_output.contains("Relevant Environment Variables:"));
        assert!(display_output.contains("Total Environment Variables: 2 entries"));
    }
}

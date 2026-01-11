// SPDX-License-Identifier: GPL-3.0-or-later

use crate::environment;
use crate::environment::KEY_OS__PATH;
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
    /// Whether preload-based interception is supported on this system
    pub preload_supported: bool,
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

        let preload_supported = is_preload_supported();

        Ok(Context { current_executable, current_directory, environment, preload_supported })
    }

    /// Returns the PATH environment variable key and value.
    ///
    /// This is relevant for Windows where the PATH is not capitalized and the lookup
    /// should be case insensitive.
    pub fn path(&self) -> Option<(String, String)> {
        self.environment
            .iter()
            .find(|(key, _)| key.to_uppercase() == KEY_OS__PATH)
            .map(|(key, value)| (key.clone(), value.clone()))
    }

    /// Parses the PATH environment variable from context into a vector of directories.
    pub fn paths(&self) -> Vec<PathBuf> {
        self.path().map(|(_, path)| std::env::split_paths(&path).collect()).unwrap_or_default()
    }
}

/// Check if preload-based interception is supported on the current platform.
///
/// Returns false if:
/// - Platform doesn't support LD_PRELOAD (e.g., Windows)
/// - macOS with System Integrity Protection (SIP) enabled
/// - Other platform-specific restrictions
fn is_preload_supported() -> bool {
    #[cfg(windows)]
    {
        // Windows doesn't support LD_PRELOAD
        false
    }
    #[cfg(all(target_os = "macos", not(windows)))]
    {
        // On macOS, check for System Integrity Protection (SIP)
        !is_sip_enabled()
    }
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    {
        // Other Unix-like systems should support preload
        true
    }
}

/// Check if System Integrity Protection (SIP) is enabled on macOS.
///
/// SIP prevents LD_PRELOAD from working with system binaries, which can cause
/// library-based interposition to fail silently.
#[cfg(target_os = "macos")]
fn is_sip_enabled() -> bool {
    use std::process::Command;

    match Command::new("csrutil").arg("status").output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .any(|line| line.contains("System Integrity Protection status:") && line.contains("enabled"))
        }
        Err(_) => {
            // If we can't run csrutil, assume SIP is disabled
            // This is a conservative approach - better to try preload and fail
            // than to unnecessarily force wrapper mode
            false
        }
    }
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Application Context:")?;
        writeln!(f, "Current Executable: {}", self.current_executable.display())?;
        writeln!(f, "Current Directory: {}", self.current_directory.display())?;
        writeln!(f, "Preload Supported: {}", self.preload_supported)?;
        writeln!(f, "Total Environment Variables: {} entries", self.environment.len())?;

        // Display relevant environment variables by iterating directly
        writeln!(f, "Relevant Environment Variables:")?;
        for (key, value) in &self.environment {
            if environment::relevant_env(key) {
                writeln!(f, "  {}={}", key, value)?;
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
            preload_supported: is_preload_supported(),
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
            preload_supported: is_preload_supported(),
        };

        let display_output = format!("{}", context);

        // Should show that there are no relevant variables
        assert!(display_output.contains("Relevant Environment Variables:"));
        assert!(display_output.contains("Total Environment Variables: 2 entries"));
    }

    #[test]
    fn test_preload_supported_field() {
        let context = Context::capture().unwrap();

        // On Windows, preload should not be supported
        #[cfg(windows)]
        assert!(!context.preload_supported);

        // On non-Windows platforms, this depends on the actual system state
        #[cfg(not(windows))]
        {
            // Just verify the field exists and has a boolean value
            let _ = context.preload_supported;
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_sip_detection() {
        // Test that SIP detection doesn't panic
        let _sip_enabled = is_sip_enabled();
    }
}

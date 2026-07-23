// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the intercept reporting and collecting functionality.
//!
//! When a command execution is intercepted, the interceptor sends the event to the collector.
//! This happens in two different processes, requiring a communication channel between these
//! processes.
//!
//! The module provides abstractions for the reporter and the collector. And it also defines
//! the data structures that are used to represent the events. It also hosts the shared
//! logging initializer (`logging`) that every Bear binary uses for a consistent diagnostic
//! format.
//!
//! # How the executable path is captured
//!
//! The `Execution.executable` field contains the path to the intercepted executable.
//! Its value depends on the interception mechanism and the exec-family function used:
//!
//! ## Wrapper mode
//!
//! The wrapper binary discovers the real compiler path at build start (via `which`) and
//! stores it in a config file. When invoked, it replaces its own path with the real
//! compiler's absolute path before reporting. **Result:** always an absolute path.
//!
//! ## Preload mode (libexec.so)
//!
//! The preload library intercepts exec-family calls and reports the path argument as-is:
//!
//! | Function    | Parameter | Typical value   | Searches PATH? |
//! |-------------|-----------|-----------------|----------------|
//! | `execve`    | `path`    | `/usr/bin/gcc`  | No             |
//! | `execv`     | `path`    | `/usr/bin/gcc`  | No             |
//! | `execvpe`   | `file`    | `gcc`           | Yes            |
//! | `execvp`    | `file`    | `gcc`           | Yes            |
//! | `execlp`    | `file`    | `gcc`           | Yes            |
//! | `posix_spawn` | `path`  | `/usr/bin/gcc`  | No             |
//! | `posix_spawnp` | `file` | `gcc`           | Yes            |
//!
//! The `p`-variant functions accept bare filenames; the preload shim reports them
//! as-is (not resolved). **Result:** absolute path or bare filename.
//!
//! ## Normalization
//!
//! None. The executable travels through semantic analysis and into the
//! compilation database exactly as reported here; bare filenames stay bare,
//! and their resolution is left to the consumer (see the
//! `output-json-compilation-database` requirement).

pub mod environment;
pub mod logging;
pub mod reporter;
pub mod state;
pub mod tcp;

use crate::environment::relevant_env;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Execution is a representation of a process execution.
///
/// It does not contain information about the outcome of the execution,
/// like the exit code or the duration of the execution. It only contains
/// the information that is necessary to reproduce the execution.
///
/// # Fields
/// - `executable`: The path to the executable that was run.
/// - `arguments`: The command line arguments that were passed to the executable.
///   Includes the executable itself as the first argument.
/// - `working_dir`: The current working directory of the process.
/// - `environment`: The environment variables that were set for the process.
#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Execution {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
}

impl Execution {
    /// Captures the execution information of the current process.
    ///
    /// This method retrieves the executable path, command-line arguments,
    /// current working directory, and environment variables of the process.
    ///
    /// **Security Note**: This method captures ALL environment variables from
    /// the current process, which may include sensitive information. Consider
    /// using the `trim()` method to filter to only relevant environment variables.
    pub fn capture() -> Result<Self, CaptureError> {
        let executable = std::env::current_exe().map_err(CaptureError::CurrentExecutable)?;
        let arguments = std::env::args().collect();
        let working_dir = std::env::current_dir().map_err(CaptureError::CurrentDirectory)?;
        let environment = std::env::vars().collect();

        Ok(Self { executable, arguments, working_dir, environment })
    }

    pub fn with_executable(self, executable: &Path) -> Self {
        Self { executable: executable.to_path_buf(), ..self }
    }

    /// Trims the execution information to only contain relevant environment variables.
    pub fn trim(self) -> Self {
        let environment = self.environment.into_iter().filter(|(k, _)| relevant_env(k)).collect();
        Self { environment, ..self }
    }

    /// Builds an `Execution` from string slices.
    ///
    /// A convenience constructor used by tests in this crate and in `bear`.
    /// It is not `#[cfg(test)]`-gated because the gate would only apply when
    /// `intercept` itself is compiled for test, leaving it unavailable to
    /// `bear`'s tests across the crate boundary.
    pub fn from_strings(
        executable: &str,
        arguments: Vec<&str>,
        working_dir: &str,
        environment: HashMap<&str, &str>,
    ) -> Self {
        Self {
            executable: PathBuf::from(executable),
            arguments: arguments.iter().map(|s| s.to_string()).collect(),
            working_dir: PathBuf::from(working_dir),
            environment: environment.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        }
    }
}

impl fmt::Display for Execution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Execution path={}, args=[{}]", self.executable.display(), self.arguments.join(","))
    }
}

/// Represents errors that can occur while capturing the execution information.
#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("Failed to capture execution: {0}")]
    CurrentExecutable(std::io::Error),
    #[error("Failed to capture current directory: {0}")]
    CurrentDirectory(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_trim() {
        // Create an execution with both relevant and irrelevant environment variables
        let environment = {
            let mut builder = HashMap::new();
            builder.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
            builder.insert("CC".to_string(), "gcc".to_string());
            builder.insert("IRRELEVANT_VAR".to_string(), "value".to_string());
            builder.insert("HOME".to_string(), "/home/user".to_string());
            builder
        };

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec!["/usr/bin/gcc".to_string(), "-c".to_string(), "test.c".to_string()],
            working_dir: PathBuf::from("/tmp"),
            environment,
        };

        let trimmed = execution.trim();

        // All environment variables in the trimmed execution should be relevant
        for key in trimmed.environment.keys() {
            assert!(relevant_env(key), "Non-relevant env var found after trim: {}", key);
        }

        // Other fields should remain unchanged
        assert_eq!(trimmed.executable, PathBuf::from("/usr/bin/gcc"));
        assert_eq!(trimmed.arguments, vec!["/usr/bin/gcc", "-c", "test.c"]);
        assert_eq!(trimmed.working_dir, PathBuf::from("/tmp"));
    }
}

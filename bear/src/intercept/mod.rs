// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the intercept reporting and collecting functionality.
//!
//! When a command execution is intercepted, the interceptor sends the event to the collector.
//! This happens in two different processes, requiring a communication channel between these
//! processes.
//!
//! The module provides abstractions for the reporter and the collector. And it also defines
//! the data structures that are used to represent the events.

pub mod environment;
pub mod reporter;
pub mod supervise;
pub mod tcp;

use crate::environment::relevant_env;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Execution is a representation of a process execution.
///
/// It does not contain information about the outcome of the execution,
/// like the exit code or the duration of the execution. It only contains
/// the information that is necessary to reproduce the execution.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
    pub fn capture() -> Result<Self, CaptureError> {
        let executable = std::env::current_exe().map_err(CaptureError::CurrentExecutable)?;
        let arguments = std::env::args().collect();
        let working_dir = std::env::current_dir().map_err(CaptureError::CurrentDirectory)?;
        let environment = std::env::vars().collect();

        Ok(Self {
            executable,
            arguments,
            working_dir,
            environment,
        })
    }

    pub fn with_executable(self, executable: &Path) -> Self {
        let mut updated = self;
        updated.executable = executable.to_path_buf();
        updated
    }

    /// Trims the execution information to only contain relevant environment variables.
    pub fn trim(self) -> Self {
        let environment = self
            .environment
            .into_iter()
            .filter(|(k, _)| relevant_env(k))
            .collect();
        Self {
            environment,
            ..self
        }
    }

    #[cfg(test)]
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
            environment: environment
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

impl fmt::Display for Execution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Execution path={}, args=[{}]",
            self.executable.display(),
            self.arguments.join(",")
        )
    }
}

/// Represent a relevant life cycle event of a process.
///
/// In the current implementation, we only have one event, the `Started` event.
/// This event is sent when a process is started. It contains the process id
/// and the execution information.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event {
    pub pid: u32,
    pub execution: Execution,
}

impl Event {
    /// Creates a new event that is originated from the current process.
    pub fn new(execution: Execution) -> Self {
        let pid = std::process::id();
        Event { pid, execution }
    }

    /// Create a new event from the current, where the new event execution
    /// is trimmed to only contain relevant environment variables.
    pub fn trim(self) -> Self {
        let execution = self.execution.trim();
        Self { execution, ..self }
    }

    #[cfg(test)]
    pub fn from_strings(
        pid: u32,
        executable: &str,
        arguments: Vec<&str>,
        working_dir: &str,
        environment: HashMap<&str, &str>,
    ) -> Self {
        Self {
            pid,
            execution: Execution::from_strings(executable, arguments, working_dir, environment),
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Event pid={}, execution={}", self.pid, self.execution)
    }
}

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("Failed to capture execution: {0}")]
    CurrentExecutable(std::io::Error),
    #[error("Failed to capture current directory: {0}")]
    CurrentDirectory(std::io::Error),
}

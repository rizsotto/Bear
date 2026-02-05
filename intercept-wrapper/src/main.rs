// SPDX-License-Identifier: GPL-3.0-or-later

//! This module implements a wrapper around an arbitrary executable.
//!
//! The wrapper is used to intercept the execution of the executable and
//! report it to a remote server. The wrapper is named after the executable
//! via a soft link (or a hard copy on platforms where soft links are not
//! supported). The wrapper process is called instead of the original executable.
//! This is arranged by the process that supervise the build process.
//! The build supervisor creates a directory with soft links and place
//! that directory at the beginning of the PATH variable. Which guarantees
//! that the wrapper is called instead of the original executable.
//!
//! The wrapper reads a JSON configuration file from the wrapper directory
//! to find the real executable path. It reports the execution of the real
//! executable and then calls the real executable with the same arguments.

extern crate core;

use anyhow::{Context, Result};
use bear::intercept::reporter::{Reporter, ReporterFactory};
use bear::intercept::supervise::supervise_execution;
use bear::intercept::wrapper::{CONFIG_FILENAME, WrapperConfig, WrapperConfigReader};
use bear::intercept::{Event, Execution};
use std::io::Write;

/// Implementation of the wrapper process.
///
/// The process exit code is the same as the executed process exit code.
/// Besides the functionality described in the module documentation, the
/// wrapper process logs the execution and the relevant steps leading to
/// the execution.
fn main() -> Result<()> {
    let pid = std::process::id();
    env_logger::Builder::from_default_env()
        .format(move |buf, record| {
            let timestamp = buf.timestamp();
            writeln!(buf, "[{timestamp} wrapper/{pid}] {}", record.args())
        })
        .init();

    // Capture the current process execution details
    let execution = Execution::capture().with_context(|| "Failed to capture the execution")?;
    // Find the real executable using JSON config
    let real_executable = find_from_config(&execution.executable)?;
    let real_execution = execution.with_executable(&real_executable);

    // Reporting failures shall not fail this process. Therefore, errors will be logged
    // but not propagated. The process will continue to execute the real executable.
    if let Err(err) = report(&real_execution) {
        log::error!("Failed to report the execution: {err}");
    }

    // Execute the real executable with the same arguments
    let exit_status = supervise_execution(real_execution)?;
    // Return the child process status code
    std::process::exit(exit_status.code().unwrap_or(1));
}

/// Report the execution to the remote collector.
fn report(real_execution: &Execution) -> Result<()> {
    let reporter = ReporterFactory::create().with_context(|| "Failed to create the reporter")?;
    // Trim environment variables when reporting to collector
    let event = Event::new(real_execution.clone()).trim();
    log::info!("Execution reported: {event:?}");
    reporter.report(event)?;

    Ok(())
}

/// Find the real executable using JSON configuration.
fn find_from_config(current_exe: &std::path::Path) -> Result<std::path::PathBuf> {
    let executable_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| "Cannot get executable name")?;

    load_config(current_exe)?
        .get_executable(executable_name)
        .cloned()
        .with_context(|| format!("Executable '{}' not found in configuration", executable_name))
}

/// Load JSON configuration file.
fn load_config(current_exe: &std::path::Path) -> Result<WrapperConfig> {
    let wrapper_dir = current_exe.parent().with_context(|| "Cannot get wrapper directory")?;

    let config_path = wrapper_dir.join(CONFIG_FILENAME);

    WrapperConfigReader::read_from_file(&config_path)
        .with_context(|| format!("Cannot read config file: {}", config_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_wrapper_config_reading() {
        use bear::intercept::wrapper::{CONFIG_FILENAME, WrapperConfig, WrapperConfigWriter};

        let temp_dir = TempDir::new().unwrap();
        let wrapper_path = temp_dir.path().join("gcc");
        let config_path = temp_dir.path().join(CONFIG_FILENAME);

        // Create a mock wrapper config
        let mut config = WrapperConfig::new();
        config.add_executable("gcc".to_string(), std::path::PathBuf::from("/usr/bin/gcc"));
        config.add_executable("g++".to_string(), std::path::PathBuf::from("/usr/bin/g++"));

        WrapperConfigWriter::write_to_file(&config, &config_path).unwrap();

        // Test reading the config
        let result = find_from_config(&wrapper_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), std::path::PathBuf::from("/usr/bin/gcc"));
    }

    #[test]
    fn test_wrapper_config_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let wrapper_path = temp_dir.path().join("gcc");

        // Test with missing config file - should fail since we only use JSON config
        let result = find_from_config(&wrapper_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot read config file"));
    }
}

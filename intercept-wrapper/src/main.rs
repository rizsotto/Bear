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
    // Load configuration
    let config = load_config(&execution.executable)?;
    // Find the real executable using config
    let real_executable = find_from_config(&config, &execution.executable)?;
    let real_execution = execution.with_executable(&real_executable);

    // Reporting failures shall not fail this process. Therefore, errors will be logged
    // but not propagated. The process will continue to execute the real executable.
    if let Err(err) = report(&config, &real_execution) {
        log::error!("Failed to report the execution: {err}");
    }

    // Execute the real executable with the same arguments
    let exit_status = supervise_execution(real_execution)?;
    // Return the child process status code
    std::process::exit(exit_status.code().unwrap_or(1));
}

/// Report the execution to the remote collector.
fn report(config: &WrapperConfig, real_execution: &Execution) -> Result<()> {
    let reporter = ReporterFactory::create(config.collector_address);
    reporter.report(Event::new(real_execution.clone()))?;

    Ok(())
}

/// Find the real executable using configuration.
fn find_from_config(config: &WrapperConfig, current_exe: &std::path::Path) -> Result<std::path::PathBuf> {
    let executable_name = current_exe
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| "Cannot get executable name")?;

    config
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

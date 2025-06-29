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
//! The wrapper reads the PATH variable and finds the next executable with
//! the same name as the wrapper. It reports the execution of the real
//! executable and then calls the real executable with the same arguments.

extern crate core;

use anyhow::{Context, Result};
use bear::intercept::reporter::{Reporter, ReporterFactory};
use bear::intercept::supervise::supervise;
use bear::intercept::{Event, Execution};

/// Implementation of the wrapper process.
///
/// The process exit code is the same as the executed process exit code.
/// Besides the functionality described in the module documentation, the
/// wrapper process logs the execution and the relevant steps leading to
/// the execution.
fn main() -> Result<()> {
    env_logger::init();
    // Capture the current process execution details
    let execution = Execution::capture().with_context(|| "Failed to capture the execution")?;
    log::info!("Execution captured: {:?}", execution);
    // Read the PATH variable and find the next executable with the same name
    let real_executable = next_in_path(&execution.executable)?;
    let real_execution = execution.with_executable(&real_executable);
    log::info!("Execution to call: {:?}", real_execution);

    // Reporting failures shall not fail this process. Therefore, errors will be logged
    // but not propagated. The process will continue to execute the real executable.
    if let Err(e) = report(&real_execution) {
        log::error!("Failed to report the execution: {}", e);
    } else {
        log::info!("Execution reported successfully");
    }

    // Execute the real executable with the same arguments
    let exit_status = supervise(real_execution)?;
    log::info!("Execution finished with status: {:?}", exit_status);
    // Return the child process status code
    std::process::exit(exit_status.code().unwrap_or(1));
}

/// Report the execution to the remote collector.
fn report(real_execution: &Execution) -> Result<()> {
    let reporter = ReporterFactory::create().with_context(|| "Failed to create the reporter")?;
    let event = Event::new(real_execution.clone());
    reporter
        .report(event)
        .with_context(|| "Failed to send report")?;

    Ok(())
}

/// Find the next executable in the PATH variable.
///
/// The function reads the PATH variable and tries to find the next executable
/// with the same name as the given executable. It returns the path to the
/// executable.
fn next_in_path(current_exe: &std::path::Path) -> Result<std::path::PathBuf> {
    let target = current_exe
        .file_name()
        .with_context(|| "Cannot get the file name of the executable")?;
    let path =
        std::env::var("PATH").with_context(|| "Cannot get the PATH variable from environment")?;

    log::debug!("PATH: {}", path);

    std::env::split_paths(&path)
        .map(|dir| dir.join(target))
        .filter(|path| path.is_file())
        .find(|path| {
            // We need to compare it with the real path of the candidate executable to avoid
            // calling the same executable again.
            let real_path = match path.canonicalize() {
                Ok(path) => path,
                Err(_) => return false,
            };
            real_path != current_exe
        })
        .ok_or_else(|| anyhow::anyhow!("Cannot find the real executable"))
}

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
use bear::intercept::create_reporter;
use bear::intercept::supervise::supervise;
use bear::intercept::{Event, Execution, ProcessId};
use std::path::{Path, PathBuf};

/// Implementation of the wrapper process.
///
/// The process exit code is the same as the executed process exit code.
/// Besides the functionality described in the module documentation, the
/// wrapper process logs the execution and the relevant steps leading to
/// the execution.
fn main() -> Result<()> {
    env_logger::init();
    // Find out what is the executable name the execution was started with
    let executable = file_name_from_arguments()?;
    log::info!("Executable as called: {:?}", executable);
    // Read the PATH variable and find the next executable with the same name
    let real_executable = next_in_path(&executable)?;
    log::info!("Executable to call: {:?}", real_executable);

    // Reporting failures shall not fail the execution.
    match into_execution(&real_executable).and_then(report) {
        Ok(_) => log::info!("Execution reported"),
        Err(e) => log::error!("Execution reporting failed: {}", e),
    }

    // Execute the real executable with the same arguments
    let mut command = std::process::Command::new(real_executable);
    let exit_status = supervise(command.args(std::env::args().skip(1)))?;
    log::info!("Execution finished with status: {:?}", exit_status);
    // Return the child process status code
    std::process::exit(exit_status.code().unwrap_or(1));
}

/// Get the file name of the executable from the arguments.
///
/// Since the executable will be called via soft link, the first argument
/// will be the name of the soft link. This function returns the file name
/// of the soft link.
fn file_name_from_arguments() -> Result<PathBuf> {
    std::env::args()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Cannot get first argument"))
        .and_then(|arg| match PathBuf::from(arg).file_name() {
            Some(file_name) => Ok(PathBuf::from(file_name)),
            None => Err(anyhow::anyhow!(
                "Cannot get the file name from the argument"
            )),
        })
}

/// Find the next executable in the PATH variable.
///
/// The function reads the PATH variable and tries to find the next executable
/// with the same name as the given executable. It returns the path to the
/// executable.
fn next_in_path(target: &Path) -> Result<PathBuf> {
    let path = std::env::var("PATH")?;
    log::debug!("PATH: {}", path);
    // The `current_exe` is a canonical path to the current executable.
    let current_exe = std::env::current_exe()?;

    path.split(':')
        .map(|dir| Path::new(dir).join(target))
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

fn report(execution: Execution) -> Result<()> {
    let event = Event {
        pid: ProcessId(std::process::id()),
        execution,
    };

    create_reporter()
        .with_context(|| "Cannot create execution reporter")?
        .report(event)
        .with_context(|| "Sending execution failed")
}

fn into_execution(path_buf: &Path) -> Result<Execution> {
    Ok(Execution {
        executable: path_buf.to_path_buf(),
        arguments: std::env::args().collect(),
        working_dir: std::env::current_dir()?,
        environment: std::env::vars().collect(),
    })
}

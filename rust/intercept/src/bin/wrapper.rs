/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate core;

use anyhow::{Context, Result};
use intercept::reporter::{Reporter, TcpReporter};
use intercept::KEY_DESTINATION;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    env_logger::init();
    // Find out what is the executable name the execution was started with
    let executable = file_name_from_arguments()?;
    log::info!("Executable as called: {:?}", executable);
    // Read the PATH variable and find the next executable with the same name
    let real_executable = next_in_path(&executable)?;
    log::info!("Executable to call: {:?}", real_executable);

    // Report the execution with the real executable
    match into_execution(&real_executable).and_then(report) {
        Ok(_) => log::info!("Execution reported"),
        Err(e) => log::error!("Execution reporting failed: {}", e),
    }

    // Execute the real executable with the same arguments
    let status = std::process::Command::new(real_executable)
        .args(std::env::args().skip(1))
        .status()?;
    log::info!("Execution finished with status: {:?}", status);
    // Return the status code
    std::process::exit(status.code().unwrap_or(1));
}

/// Get the file name of the executable from the arguments.
///
/// Since the executable will be called via soft link, the first argument
/// will be the name of the soft link. This function returns the file name
/// of the soft link.
fn file_name_from_arguments() -> Result<PathBuf> {
    std::env::args().next()
        .ok_or_else(|| anyhow::anyhow!("Cannot get first argument"))
        .and_then(|arg| match PathBuf::from(arg).file_name() {
            Some(file_name) => Ok(PathBuf::from(file_name)),
            None => Err(anyhow::anyhow!("Cannot get the file name from the argument")),
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
        .filter(|path| path.is_file()) // TODO: check if it is executable
        .filter(|path| {
            // We need to compare it with the real path of the candidate executable to avoid
            // calling the same executable again.
            let real_path = match path.canonicalize() {
                Ok(path) => path,
                Err(_) => return false,
            };
            real_path != current_exe
        })
        .nth(0)
        .ok_or_else(|| anyhow::anyhow!("Cannot find the real executable"))
}

fn report(execution: intercept::Execution) -> Result<()> {
    let event = intercept::Event {
        pid: intercept::ProcessId(std::process::id() as u32),
        execution,
    };

    // Get the reporter address from the environment
    std::env::var(KEY_DESTINATION)
        .with_context(|| format!("${} is missing from the environment", KEY_DESTINATION))
        // Create a new reporter
        .and_then(|reporter_address| TcpReporter::new(reporter_address))
        .with_context(|| "Cannot create TCP execution reporter")
        // Report the execution
        .and_then(|reporter| reporter.report(event))
        .with_context(|| "Sending execution failed")
}

fn into_execution(path_buf: &Path) -> Result<intercept::Execution> {
    std::env::current_dir()
        .with_context(|| "Cannot get current directory")
        .map(|working_dir| intercept::Execution {
            executable: path_buf.to_path_buf(),
            arguments: std::env::args().collect(),
            working_dir,
            environment: std::env::vars().collect(),
        })
}

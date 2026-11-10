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

use anyhow::{Context, Result, bail};
use intercept::Execution;
use intercept::reporter::{Reporter, ReporterFactory};
use intercept_supervisor::supervise::{Inherit, signals, supervise};
use intercept_supervisor::wrapper::{CONFIG_FILENAME, WrapperConfig, WrapperConfigReader};

/// Implementation of the wrapper process.
///
/// The process exit code is the same as the executed process exit code.
/// Besides the functionality described in the module documentation, the
/// wrapper process logs the execution and the relevant steps leading to
/// the execution.
fn main() -> Result<()> {
    intercept::logging::init("wrapper");
    // Register before anything is spawned, so no signal is lost.
    let signals = signals().with_context(|| "Failed to register signal handlers")?;

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
    let mut command = command_from_execution(real_execution)?;
    let exit_status = supervise::<Inherit>(&mut command, signals)?;
    // Return the child process status code
    std::process::exit(exit_status.code().unwrap_or(1));
}

/// Builds a [`std::process::Command`] from an [`Execution`].
///
/// This is a free function rather than a `TryFrom` impl because the orphan
/// rule forbids implementing the foreign `TryFrom`/`Command` for the foreign
/// `Execution` from this crate.
fn command_from_execution(execution: Execution) -> Result<std::process::Command> {
    let mut command = match execution.arguments.as_slice() {
        [] => bail!("Execution arguments cannot be empty"),
        [_] => std::process::Command::new(execution.executable),
        [_, arguments @ ..] => {
            let mut command = std::process::Command::new(execution.executable);
            command.args(arguments);
            command
        }
    };

    command.envs(execution.environment);
    command.current_dir(execution.working_dir);
    Ok(command)
}

/// Report the execution to the remote collector.
fn report(config: &WrapperConfig, real_execution: &Execution) -> Result<()> {
    let reporter = ReporterFactory::create(config.collector_address);
    reporter.report(real_execution.clone())?;

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

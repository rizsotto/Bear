// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::Execution;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time;
use thiserror::Error;

/// This method supervises the execution of a command.
///
/// It starts the command and waits for its completion. It also forwards
/// signals to the child process. The method returns the exit status of the
/// child process.
pub fn supervise(command: &mut std::process::Command) -> Result<ExitStatus, SuperviseError> {
    let executable = PathBuf::from(command.get_program());
    let signaled = Arc::new(AtomicUsize::new(0));
    for signal in signal_hook::consts::TERM_SIGNALS {
        signal_hook::flag::register_usize(*signal, Arc::clone(&signaled), *signal as usize).map_err(
            |err| SuperviseError::SignalRegistration { executable: executable.clone(), source: err },
        )?;
    }

    let mut child = command
        .spawn()
        .map_err(|err| SuperviseError::ProcessSpawn { executable: executable.clone(), source: err })?;

    loop {
        // Forward signals to the child process, but don't exit the loop while it is running
        if signaled.swap(0usize, Ordering::SeqCst) != 0 {
            log::debug!("Received signal, forwarding to child process");
            child
                .kill()
                .map_err(|err| SuperviseError::ProcessKill { executable: executable.clone(), source: err })?;
        }

        // Check if the child process has exited
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                log::debug!("Child process exited: {exit_status:?}");
                return Ok(exit_status);
            }
            Ok(None) => {
                thread::sleep(time::Duration::from_millis(100));
            }
            Err(err) => {
                log::error!("Error waiting for child process: {err}");
                return Err(SuperviseError::ProcessWait { executable: executable.clone(), source: err });
            }
        }
    }
}

/// This function supervises the execution of a command represented by the `Execution` struct.
pub fn supervise_execution(execution: Execution) -> Result<ExitStatus, SuperviseError> {
    let mut child = Into::<std::process::Command>::into(execution);
    supervise(&mut child)
}

impl From<Execution> for std::process::Command {
    fn from(val: Execution) -> Self {
        let mut command = match val.arguments.as_slice() {
            [] => panic!("Execution arguments cannot be empty"),
            [_] => std::process::Command::new(val.executable),
            [_, arguments @ ..] => {
                let mut cmd = std::process::Command::new(val.executable);
                cmd.args(arguments);
                cmd
            }
        };

        command.envs(val.environment);
        command.current_dir(val.working_dir);
        command
    }
}

/// Errors that can occur during process supervision.
#[derive(Error, Debug)]
pub enum SuperviseError {
    #[error("Failed to register signal handler for '{executable}': {source}", executable = executable.display())]
    SignalRegistration {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to execute '{executable}': {source}", executable = executable.display())]
    ProcessSpawn {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to kill process '{executable}': {source}", executable = executable.display())]
    ProcessKill {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to wait for process '{executable}': {source}", executable = executable.display())]
    ProcessWait {
        executable: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

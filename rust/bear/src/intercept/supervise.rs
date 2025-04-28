// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::Execution;
use anyhow::Result;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time;

/// This method supervises the execution of a command.
///
/// It starts the command and waits for its completion. It also forwards
/// signals to the child process. The method returns the exit status of the
/// child process.
pub fn supervise(execution: Execution) -> Result<ExitStatus> {
    let signaled = Arc::new(AtomicUsize::new(0));
    for signal in signal_hook::consts::TERM_SIGNALS {
        signal_hook::flag::register_usize(*signal, Arc::clone(&signaled), *signal as usize)?;
    }

    let mut child = Into::<std::process::Command>::into(execution).spawn()?;
    loop {
        // Forward signals to the child process, but don't exit the loop while it is running
        if signaled.swap(0usize, Ordering::SeqCst) != 0 {
            log::debug!("Received signal, forwarding to child process");
            child.kill()?;
        }

        // Check if the child process has exited
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                log::debug!("Child process exited");
                return Ok(exit_status);
            }
            Ok(None) => {
                thread::sleep(time::Duration::from_millis(100));
            }
            Err(e) => {
                log::error!("Error waiting for child process: {}", e);
                return Err(e.into());
            }
        }
    }
}

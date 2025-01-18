// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use nix::libc::c_int;
#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
#[cfg(windows)]
use winapi::shared::minwindef::FALSE;
#[cfg(windows)]
use winapi::um::processthreadsapi::{OpenProcess, TerminateProcess};
#[cfg(windows)]
use winapi::um::winnt::{PROCESS_TERMINATE, SYNCHRONIZE};

/// This method supervises the execution of a command.
///
/// It starts the command and waits for its completion. It also forwards
/// signals to the child process. The method returns the exit status of the
/// child process.
pub fn supervise(command: &mut Command) -> Result<ExitStatus> {
    let mut child = command.spawn()?;

    let child_pid = child.id();
    let running = Arc::new(AtomicBool::new(true));
    let running_in_thread = running.clone();

    let mut signals = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
    ])?;

    #[cfg(unix)]
    {
        signals.add_signal(signal_hook::consts::SIGHUP)?;
        signals.add_signal(signal_hook::consts::SIGQUIT)?;
        signals.add_signal(signal_hook::consts::SIGALRM)?;
        signals.add_signal(signal_hook::consts::SIGUSR1)?;
        signals.add_signal(signal_hook::consts::SIGUSR2)?;
        signals.add_signal(signal_hook::consts::SIGCONT)?;
        signals.add_signal(signal_hook::consts::SIGSTOP)?;
    }

    let handler = thread::spawn(move || {
        for signal in signals.forever() {
            log::debug!("Received signal: {:?}", signal);
            if forward_signal(signal, child_pid) {
                // If the signal caused termination, we should stop the process.
                running_in_thread.store(false, Ordering::SeqCst);
                break;
            }
        }
    });

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }
    handler.join().unwrap();

    let exit_status = child.wait()?;

    Ok(exit_status)
}

#[cfg(windows)]
fn forward_signal(_: c_int, child_pid: u32) -> bool {
    let process_handle = unsafe { OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, FALSE, child_pid) };
    if process_handle.is_null() {
        let err = unsafe { winapi::um::errhandling::GetLastError() };
        log::error!("Failed to open process: {}", err);
        // If the process handle is not valid, presume the process is not running anymore.
        return true;
    }

    let terminated = unsafe { TerminateProcess(process_handle, 1) };
    if terminated == FALSE {
        let err = unsafe { winapi::um::errhandling::GetLastError() };
        log::error!("Failed to terminate process: {}", err);
    }

    // Ensure proper handle closure
    unsafe { winapi::um::handleapi::CloseHandle(process_handle) };

    // Return true if the process was terminated.
    terminated == TRUE
}

#[cfg(unix)]
fn forward_signal(signal: c_int, child_pid: u32) -> bool {
    // Forward the signal to the child process
    if let Err(e) = kill(
        Pid::from_raw(child_pid as i32),
        Signal::try_from(signal).ok(),
    ) {
        log::error!("Error forwarding signal: {}", e);
    }

    // Return true if the process was terminated.
    match kill(Pid::from_raw(child_pid as i32), None) {
        Ok(_) => {
            log::debug!("Checking if the process is still running... yes");
            false
        }
        Err(nix::Error::ESRCH) => {
            log::debug!("Checking if the process is still running... no");
            true
        }
        Err(_) => {
            log::debug!("Checking if the process is still running... presume dead");
            true
        }
    }
}

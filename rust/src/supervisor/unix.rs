/*  Copyright (C) 2012-2018 by László Nagy
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

use std::env;
use std::ffi;
use std::path;
use std::process;
use std::str;

use chrono;

use crate::{Error, ErrorKind, Result, ResultExt};
use crate::event::{Event, ExitCode, ProcessId};

pub struct Supervisor<F>
    where F: FnMut(Event) -> ()
{
    sink: F,
}

impl<F> Supervisor<F>
    where F: FnMut(Event) -> ()
{
    pub fn new(sink: F) -> Supervisor<F> {
        Supervisor { sink }
    }

    pub fn run(&mut self, cmd: &[String]) -> Result<ExitCode> {
        let cwd = env::current_dir()
            .chain_err(|| "Unable to get current working directory")?;

        spawn(cmd)
            .map_err(|err| Error::with_chain(err, format!("Failed to execute: {}", cmd[0])))
            .and_then(|pid| {
                (self.sink)(
                    Event::Created {
                        pid: pid.as_raw() as ProcessId,
                        ppid: nix::unistd::Pid::parent().as_raw() as ProcessId,
                        cwd: cwd.clone(),
                        cmd: cmd.to_vec(),
                        when: chrono::Utc::now(),
                    });
                self.wait_for_pid(pid)
            })
    }

    fn wait_for_pid(&mut self, child: nix::unistd::Pid) -> Result<ExitCode> {
        match nix::sys::wait::waitpid(child, None) {
            Ok(nix::sys::wait::WaitStatus::Exited(pid, code)) => {
                (self.sink)(
                    Event::TerminatedNormally {
                        pid: pid.as_raw() as ProcessId,
                        code,
                        when: chrono::Utc::now(),
                    });
                Ok(code)
            },
            Ok(nix::sys::wait::WaitStatus::Signaled(pid, signal, bool)) => {
                (self.sink)(
                    Event::TerminatedAbnormally {
                        pid: pid.as_raw() as ProcessId,
                        signal: format!("{}", signal),
                        when: chrono::Utc::now(),
                    });
                // TODO: fake the signal in return value.
                Ok(1)
            },
            Ok(nix::sys::wait::WaitStatus::Stopped(pid, signal)) => {
                (self.sink)(
                    Event::Stopped {
                        pid: pid.as_raw() as ProcessId,
                        signal: format!("{}", signal),
                        when: chrono::Utc::now(),
                    });
                self.wait_for_pid(child)
            },
            Ok(nix::sys::wait::WaitStatus::Continued(pid)) => {
                (self.sink)(
                    Event::Continued {
                        pid: pid.as_raw() as ProcessId,
                        when: chrono::Utc::now(),
                    });
                self.wait_for_pid(child)
            },
            Ok(_) => {
                info!("Wait status is ignored, continue to wait.");
                self.wait_for_pid(child)
            },
            Err(error) =>
                Err(Error::with_chain(error, "Process creation failed.")),
        }
    }
}

fn spawn(cmd: &[String]) -> Result<nix::unistd::Pid> {
    // Create communication channel between the child and parent processes.
    // Parent want to be notified if process execution went well or failed.
    let (read_fd, write_fd) = nix::unistd::pipe()
        .chain_err(|| "Unable to create pipe.")?;

    match nix::unistd::fork() {
        Ok(nix::unistd::ForkResult::Parent { child, .. }) => {
            debug!("Parent process: waiting for pid: {}", child);
            let _ = nix::unistd::close(write_fd);

            let mut buffer = vec![0u8; 1024];
            match nix::unistd::read(read_fd, buffer.as_mut()) {
                Ok(0) => {
                    // In case of successful start the child closed the pipe,
                    // so we can't read anything from it.
                    let _ = nix::unistd::close(read_fd);
                    Ok(child)
                },
                Ok(length) => {
                    // If the child failed to exec the given process,
                    // it sends us a message through the pipe.
                    let _ = nix::unistd::close(read_fd);
                    // Take that read value and use as error message.
                    Err(
                        str::from_utf8(buffer.as_ref())
                            .unwrap_or("Unknown reason.")
                            .into())
                },
                Err(error) => {
                    let _ = nix::unistd::close(read_fd);
                    Err(Error::with_chain(error, "Read from pipe failed."))
                },
            }
        }
        Ok(nix::unistd::ForkResult::Child) => {
            debug!("Child process: calling exec.");
            let _ = nix::unistd::close(read_fd);

            let args : Vec<_> = cmd.iter()
                .map(|arg| ffi::CString::new(arg.as_bytes()).unwrap())
                .collect();
            nix::unistd::execvp(&args[0], args.as_ref())
                .map_err(|error| {
                    let message = error.to_string().into_bytes();
                    let _ = nix::unistd::write(write_fd, message.as_ref());
                    let _ = nix::unistd::close(write_fd);
                });
            process::exit(1);
        },
        Err(error) =>
            Err(Error::with_chain(error, "Fork process failed.")),
    }
}

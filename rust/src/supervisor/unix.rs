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

use chrono;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, execvp, fork, ForkResult, Pid, pipe, read, write};

use crate::{Error, ErrorKind, Result, ResultExt};
use crate::event::*;

use super::fake::get_parent_pid;

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
            .chain_err(|| "unable to get current working directory")?;

        spawn(cmd)
            .and_then(|pid| {
                (self.sink)(
                    Event::Created {
                        pid: pid.as_raw() as ProcessId,
                        ppid: Pid::parent().as_raw() as ProcessId,
                        cwd: cwd.clone(),
                        cmd: cmd.to_vec(),
                        when: chrono::Utc::now(),
                    });
                self.wait_for_pid(pid)
            })
    }

    fn wait_for_pid(&mut self, child: Pid) -> Result<ExitCode> {
        waitpid(child, None)
            .map_err(|err| err.into())
            .and_then(|status|
                match status {
                    WaitStatus::Exited(pid, code) => {
                        debug!("exited");
                        (self.sink)(
                            Event::TerminatedNormally {
                                pid: pid.as_raw() as ProcessId,
                                code,
                                when: chrono::Utc::now(),
                            });
                        Ok(code)
                    },
                    WaitStatus::Signaled(pid, signal, bool) => {
                        debug!("signaled");
                        (self.sink)(
                            Event::TerminatedAbnormally {
                                pid: pid.as_raw() as ProcessId,
                                signal: format!("{}", signal),
                                when: chrono::Utc::now(),
                            });
                        // TODO: fake the signal in return value.
                        Ok(1)
                    },
                    WaitStatus::Stopped(pid, signal) => {
                        debug!("stopped");
                        (self.sink)(
                            Event::Stopped {
                                pid: pid.as_raw() as ProcessId,
                                signal: format!("{}", signal),
                                when: chrono::Utc::now(),
                            });
                        self.wait_for_pid(child)
                    },
                    WaitStatus::Continued(pid) => {
                        debug!("continued");
                        (self.sink)(
                            Event::Continued {
                                pid: pid.as_raw() as ProcessId,
                                when: chrono::Utc::now(),
                            });
                        self.wait_for_pid(child)
                    },
                    _ => {
                        info!("Wait status is ignored, continue to wait.");
                        self.wait_for_pid(child)
                    },
                }
            )
            .chain_err(|| "Process creation failed.")
    }
}

fn spawn(cmd: &[String]) -> Result<Pid> {
    let (read_fd, write_fd) = pipe()?;

    fork()
        .map_err(|err| err.into())
        .and_then(|fork_result| {
            match fork_result {
                ForkResult::Parent { child, .. } => {
                    debug!("Parent process: waiting for pid: {}", child);
                    close(write_fd);

                    let mut buffer = vec![0u8, 10];
                    read(read_fd, buffer.as_mut())
                        .map_err(|err| Error::with_chain(err, "Read from pipe failed."))
                        .and_then(|length| {
                            if length == 0 {
                                // In case of successful start the child closed the pipe,
                                // so we can't read anything from it.
                                Ok(child)
                            } else {
                                // If the child failed to exec the given process, it
                                // sends us a message through the pipe.
                                bail!("Could not execute process: {}", cmd[0])
                            }
                        })
                }
                ForkResult::Child => {
                    debug!("Child process: will call exec soon.");
                    close(read_fd);

                    let args : Vec<_> = cmd.iter()
                        .map(|arg| ffi::CString::new(arg.as_bytes()).unwrap())
                        .collect();
                    execvp(&args[0], args.as_ref())
                        .map_err(|_| {
                            // TODO: send the error message through
                            write(write_fd, b"error");
                            close(write_fd);
                        });
                    ::std::process::exit(1);
                },
            }
        })
}

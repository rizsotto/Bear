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
use nix::unistd::{fork, execvp, pipe, close, write, read, ForkResult, Pid};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};

use crate::{ErrorKind, Result, ResultExt};
use crate::event::*;
use super::fake::get_parent_pid;


pub struct Supervisor<'a> {
    sink: Box<FnMut(Event) -> Result<()> + 'a>,
}

impl<'a> Supervisor<'a> {
    pub fn new<F: 'a>(sink: F) -> Supervisor<'a>
        where F: FnMut(Event) -> Result<()> {
        Supervisor { sink: Box::new(sink) }
    }

    pub fn run(&mut self, cmd: &[String]) -> Result<ExitCode> {
        let cwd = env::current_dir()
            .chain_err(|| "unable to get current working directory")?;

        match spawn(cmd) {
            Ok(pid) => {
                let event = Event::Created(
                    ProcessCreated {
                        pid: pid.as_raw() as ProcessId,
                        ppid: Pid::parent().as_raw() as ProcessId,
                        cwd: cwd.clone(),
                        cmd: cmd.to_vec(),
                    },
                    chrono::Utc::now());
                self.report(event);
                self.wait_for_pid(pid)
            },
            Err(_) => {
                bail!("Could not execute process")
            },
        }
    }

    fn wait_for_pid(&mut self, child: Pid) -> Result<ExitCode> {
        waitpid(child, None)
            .map_err(|err| ErrorKind::RuntimeError("waitpid failed").into())
            .and_then(|status|
                match status {
                    WaitStatus::Exited(pid, code) => {
                        debug!("exited");
                        let event = Event::TerminatedNormally(
                            ProcessTerminated { pid: pid.as_raw() as ProcessId, code },
                            chrono::Utc::now());
                        self.report(event);
                        Ok(code)
                    },
                    WaitStatus::Signaled(pid, signal, bool) => {
                        debug!("signaled");
                        let event = Event::TerminatedAbnormally(
                            ProcessSignaled {
                                pid: pid.as_raw() as ProcessId,
                                signal: format!("{}", signal)
                            },
                            chrono::Utc::now());
                        self.report(event);
                        Ok(-1) // TODO: check
                    },
                    WaitStatus::Stopped(pid, signal) => {
                        debug!("stopped");
                        // TODO: send event
                        self.wait_for_pid(child)
                    },
                    WaitStatus::PtraceEvent(pid, signal, c_int) => {
                        debug!("ptrace-event");
                        self.wait_for_pid(child)
                    },
                    WaitStatus::PtraceSyscall(pid) => {
                        debug!("ptrace-systrace");
                        self.wait_for_pid(child)
                    },
                    WaitStatus::Continued(pid) => {
                        debug!("continued");
                        // TODO: send event
                        self.wait_for_pid(child)
                    },
                    WaitStatus::StillAlive => {
                        debug!("still alive");
                        self.wait_for_pid(child)
                    },
                }
            )
            .chain_err(|| "Process creation failed.")
    }

    fn report(&mut self, event: Event) {
        match (self.sink)(event) {
            Ok(_) => debug!("Event sent."),
            Err(error) => debug!("Event sending failed. {:?}", error),
        }
    }
}

fn spawn(cmd: &[String]) -> Result<Pid> {
    let (read_fd, write_fd) = pipe()?;

    match fork() {
        Ok(ForkResult::Parent { child, .. }) => {
            debug!("Parent process: waiting for pid: {}", child);
            close(write_fd);

            let mut buffer = vec![0u8, 10];
            match read(read_fd, buffer.as_mut()) {
                // In case of successful start the child closed the pipe,
                // so we can't read anything from it.
                Ok(0) =>
                    Ok(child),
                // If the child failed to exec the given process, it
                // sends us a message through the pipe.
                Ok(length) =>
                    bail!("Could not execute process. {:?}", cmd[0]),
                // Not sure if this will happen, when we can't read.
                Err(_) =>
                    bail!("Read failed."),
            }
        }
        Ok(ForkResult::Child) => {
            debug!("Child process: will call exec soon.");
            close(read_fd);

            let args : Vec<_> = cmd.iter()
                .map(|arg| ffi::CString::new(arg.as_bytes()).unwrap())
                .collect();
            execvp(&args[0], args.as_ref())
                .map(|_| /* Not going to execute this. */ 0)
                .map_err(|_| {
                    write(write_fd, b"error");
                });
            bail!("exec failed: {:?}", cmd)
        },
        Err(_) =>
            bail!("Could not fork process"),
    }
}

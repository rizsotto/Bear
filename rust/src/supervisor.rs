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
use std::path;
use std::process;
use std::str;

use chrono;

use crate::{Result, ResultExt};
use crate::event::{Event, ExitCode, ProcessId};

trait Executor {
    type Handle;

    fn spawn<F>(sink: &mut F, cmd: &[String], cwd: path::PathBuf) -> Result<Self::Handle>
        where F: FnMut(Event) -> ();

    fn wait<F>(sink: &mut F, handle: &mut Self::Handle) -> Result<ExitCode>
        where F: FnMut(Event) -> ();

    fn run<F>(sink: &mut F, cmd: &[String]) -> Result<ExitCode>
        where F: FnMut(Event) -> ()
    {
        let cwd = env::current_dir()
            .chain_err(|| "Unable to get current working directory")?;

        let mut child = Self::spawn(sink, cmd, cwd)
            .chain_err(|| format!("Unable to execute: {}", &cmd[0]))?;
        Self::wait(sink, &mut child)
    }
}

pub struct Supervisor<F>
    where F: FnMut(Event) -> ()
{
    sink: F,
}

impl<F> Supervisor<F>
    where F: FnMut(Event) -> ()
{
    pub fn new(sink: F) -> Supervisor<F>
    {
        Supervisor { sink }
    }

    pub fn run(&mut self, cmd: &[String]) -> Result<ExitCode>
    {
        if cfg!(unix) {
            unix::ProcessHandle::run(&mut self.sink, cmd)
        } else {
            generic::ProcessHandle::run(&mut self.sink, cmd)
        }
    }
}

pub fn get_parent_pid() -> ProcessId {
    if cfg!(unix) {
        std::os::unix::process::parent_id()
    } else {
        match env::var("INTERCEPT_PPID") {
            Ok(value) => {
                match value.parse() {
                    Ok(ppid) => ppid,
                    _ => 0,
                }
            },
            _ => 0,
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::ffi;
    use nix::unistd;
    use nix::sys::wait;

    use super::*;
    use crate::{Error, Result, ResultExt};
    use crate::event::{Event, ProcessId};

    pub struct ProcessHandle {
        pid: nix::unistd::Pid,
    }

    impl Executor for ProcessHandle {
        type Handle = ProcessHandle;

        fn spawn<F>(sink: &mut F, cmd: &[String], cwd: path::PathBuf) -> Result<Self::Handle>
            where F: FnMut(Event) -> ()
        {
            spawn(cmd)
                .and_then(|pid| {
                    sink(
                        Event::Created {
                            pid: pid.as_raw() as ProcessId,
                            ppid: nix::unistd::Pid::parent().as_raw() as ProcessId,
                            cwd: cwd.clone(),
                            cmd: cmd.to_vec(),
                            when: chrono::Utc::now(),
                        });
                    Ok(ProcessHandle { pid })
                })
        }

        fn wait<F>(sink: &mut F, handle: &mut Self::Handle) -> Result<ExitCode>
            where F: FnMut(Event) -> ()
        {
            match wait::waitpid(handle.pid, None) {
                Ok(wait::WaitStatus::Exited(pid, code)) => {
                    sink(
                        Event::TerminatedNormally {
                            pid: pid.as_raw() as ProcessId,
                            code,
                            when: chrono::Utc::now(),
                        });
                    Ok(code)
                },
                Ok(wait::WaitStatus::Signaled(pid, signal, _dump)) => {
                    sink(
                        Event::TerminatedAbnormally {
                            pid: pid.as_raw() as ProcessId,
                            signal: format!("{}", signal),
                            when: chrono::Utc::now(),
                        });
                    Ok(127)
                },
                Ok(wait::WaitStatus::Stopped(pid, signal)) => {
                    sink(
                        Event::Stopped {
                            pid: pid.as_raw() as ProcessId,
                            signal: format!("{}", signal),
                            when: chrono::Utc::now(),
                        });
                    Self::wait(sink, handle)
                },
                Ok(wait::WaitStatus::Continued(pid)) => {
                    sink(
                        Event::Continued {
                            pid: pid.as_raw() as ProcessId,
                            when: chrono::Utc::now(),
                        });
                    Self::wait(sink, handle)
                },
                Ok(_) => {
                    info!("Wait status is ignored, continue to wait.");
                    Self::wait(sink, handle)
                },
                Err(error) =>
                    Err(Error::with_chain(error, "Process creation failed.")),
            }
        }
    }

    fn spawn(cmd: &[String]) -> Result<nix::unistd::Pid> {
        // Create communication channel between the child and parent processes.
        // Parent want to be notified if process execution went well or failed.
        let (read_fd, write_fd) = unistd::pipe()
            .chain_err(|| "Unable to create pipe.")?;

        match unistd::fork() {
            Ok(unistd::ForkResult::Parent { child, .. }) => {
                debug!("Parent process: waiting for pid: {}", child);
                let _ = unistd::close(write_fd);
                defer! {{ let _ = unistd::close(read_fd); }}

                let mut buffer = vec![0u8; 1024];
                match unistd::read(read_fd, buffer.as_mut()) {
                    Ok(0) => {
                        // In case of successful start the child closed the pipe,
                        // so we can't read anything from it.
                        debug!("Parent process: looks the child was done well.");
                        Ok(child)
                    },
                    Ok(_) => {
                        // If the child failed to exec the given process,
                        // it sends us a message through the pipe.
                        // Take that read value and use as error message.
                        debug!("Parent process: looks the child failed exec.");
                        Err(
                            str::from_utf8(buffer.as_ref())
                                .unwrap_or("Unknown reason.")
                                .into())
                    },
                    Err(error) =>
                        Err(Error::with_chain(error, "Read from pipe failed.")),
                }
            }
            Ok(unistd::ForkResult::Child) => {
                debug!("Child process: calling exec.");
                let _ = unistd::close(read_fd);
                defer! {{ let _ = unistd::close(write_fd); }}

                let args: Vec<_> = cmd.iter()
                    .map(|arg| ffi::CString::new(arg.as_bytes()).unwrap())
                    .collect();
                unistd::execvp(&args[0], args.as_ref())
                    .map_err(|error| {
                        let message = error.to_string().into_bytes();
                        let _ = unistd::write(write_fd, message.as_ref());
                    });
                debug!("Child process: exec failed, calling exit.");
                process::exit(1);
            },
            Err(error) =>
                Err(Error::with_chain(error, "Fork process failed.")),
        }
    }
}

mod generic {
    use super::*;
    use crate::{Error, Result, ResultExt};
    use crate::event::{Event, ProcessId};

    pub struct ProcessHandle {
        child: process::Child,
    }

    impl Executor for ProcessHandle {
        type Handle = ProcessHandle;

        fn spawn<F>(sink: &mut F, cmd: &[String], cwd: path::PathBuf) -> Result<Self::Handle>
            where F: FnMut(Event) -> ()
        {
            let child = process::Command::new(&cmd[0]).args(&cmd[1..]).spawn()
                .chain_err(|| format!("unable to execute process: {:?}", cmd[0]))?;

            sink(
                Event::Created {
                    pid: child.id() as ProcessId,
                    ppid: get_parent_pid(),
                    cwd: cwd.clone(),
                    cmd: cmd.to_vec(),
                    when: chrono::Utc::now(),
                });

            Ok(ProcessHandle { child })
        }

        fn wait<F>(sink: &mut F, handle: &mut Self::Handle) -> Result<ExitCode>
            where F: FnMut(Event) -> ()
        {
            match handle.child.wait() {
                Ok(status) => {
                    match status.code() {
                        Some(code) => {
                            sink(
                                Event::TerminatedNormally {
                                    pid: handle.child.id(),
                                    code,
                                    when: chrono::Utc::now(),
                                });
                            Ok(code)
                        }
                        None => {
                            sink(
                                Event::TerminatedAbnormally {
                                    pid: handle.child.id(),
                                    signal: "unknown".to_string(),
                                    when: chrono::Utc::now(),
                                });
                            Ok(127)
                        }
                    }
                }
                Err(error) => {
                    warn!("Child process was not running: {:?}", handle.child.id());
                    Err(Error::with_chain(error, "Waiting for process failed."))
                }
            }
        }
    }
}

mod fake {
    use super::*;
    use crate::Result;
    use crate::event::{Event, ProcessId};

    pub struct ProcessHandle { }

    impl Executor for ProcessHandle {
        type Handle = ProcessHandle;

        fn spawn<F>(sink: &mut F, cmd: &[String], cwd: path::PathBuf) -> Result<Self::Handle>
            where F: FnMut(Event) -> ()
        {
            sink(Event::Created {
                pid: process::id() as ProcessId,
                ppid: get_parent_pid(),
                cwd: cwd.clone(),
                cmd: cmd.to_vec(),
                when: chrono::Utc::now(),
            });

            Ok(ProcessHandle { })
        }

        fn wait<F>(sink: &mut F, _: &mut Self::Handle) -> Result<ExitCode>
            where F: FnMut(Event) -> ()
        {
            match fake_execution() {
                Ok(_) => {
                    sink(
                        Event::TerminatedNormally {
                            pid: process::id() as ProcessId,
                            code: 0,
                            when:  chrono::Utc::now(),
                        });
                    Ok(0)
                },
                Err(_) => {
                    sink(
                        Event::TerminatedNormally {
                            pid: process::id() as ProcessId,
                            code: 1,
                            when:  chrono::Utc::now(),
                        });
                    Ok(1)
                }
            }
        }
    }

    /// The main responsibility is to fake the program execution by making the
    /// relevant side effects.
    ///
    /// For a compiler, linker call the expected side effect by the build system
    /// is to create the output files. That will make sure that the build tool
    /// will continue the build process.
    fn fake_execution() -> Result<()> {
        unimplemented!()
    }
}

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

use chrono;

use std::env;
use std::path;
use std::process;
use std::sync::mpsc;

use ErrorKind;
use Result;

use event::*;

#[cfg(unix)]
fn get_parent_pid() -> Result<ProcessId> {
    let ppid: libc::pid_t = unsafe { libc::getppid() };
    Ok(ppid as ProcessId)
}

#[cfg(not(unix))]
fn get_parent_pid() -> Result<ProcessId> {
    Ok(process::id())
}

pub struct EventSender {
    sender: mpsc::Sender<Event>,
}

impl EventSender {
    pub fn report_started(&self, cmd: &[String], pid: ProcessId) {
        fn started_event(cmd: &[String], pid: ProcessId) -> Result<Event> {
            let detail = ProcessStarted {
                pid,
                ppid: get_parent_pid()?,
                cwd: env::current_dir()?,
                cmd: cmd.to_vec(),
            };

            Ok(Event::Started(detail, chrono::Utc::now()))
        }

        // TODO: write log message about the failure
        started_event(cmd, pid)
            .and_then(|event| {
                self.send_report(event);
                Ok(())
            });
    }

    pub fn report_failed(&self, cmd: &[String], error: String) {
        fn failed_event(cmd: &[String], error: String) -> Result<Event> {
            let detail = ProcessStartFailed {
                cwd: env::current_dir()?,
                cmd: cmd.to_vec(),
                error,
            };

            Ok(Event::Failed(detail, chrono::Utc::now()))
        }

        // TODO: write log message about the failure
        failed_event(cmd, error)
            .and_then(|event| {
                self.send_report(event);
                Ok(())
            });
    }

    #[cfg(unix)]
    pub fn report_status(&self, pid: ProcessId, status: &process::ExitStatus) {
        use ::std::os::unix::process::ExitStatusExt;

        match status.signal() {
            Some(number) => self.report_signaled(pid, number),
            None => self.report_stopped(pid, status),
        }
    }

    #[cfg(not(unix))]
    pub fn report_status(&self, pid: ProcessId, status: &process::ExitStatus) {
        self.report_stopped(pid, status)
    }

    fn report_stopped(&self, pid: ProcessId, status: &process::ExitStatus) {
        let exit_code = match status.code() {
            // Report the received status code.
            Some(number) => number,
            // Report something, otherwise it's considered as a running one.
            None =>  -1,
        };

        let detail = ProcessStopped { pid, exit_code };
        let event = Event::Stopped(detail, chrono::Utc::now());

        self.send_report(event)
    }

    fn report_signaled(&self, pid: ProcessId, signal: SignalId) {
        let detail = ProcessSignaled { pid, signal };
        let event = Event::Signaled(detail, chrono::Utc::now());

        self.send_report(event)
    }

    fn send_report(&self, event: Event) {
        // TODO: write log message about the failure
        match self.sender.send(event) {
            Ok(_) => (),
            Err(_) => (),
        }
    }
}

pub fn run(cmd: &[String], sender: &EventSender) -> Result<()> {
    let mut command = process::Command::new(&cmd[0]);
    match command.args(&cmd[1..]).spawn() {
        Ok(mut child) => {
            let pid: ProcessId = child.id();

            sender.report_started(cmd, pid);
            match child.try_wait() {
                Ok(Some(status)) => {
                    sender.report_status(pid, &status);
                    if status.success() {
                        Ok(())
                    } else {
                        bail!(ErrorKind::RuntimeError("process exited with non zero status"));
                    }
                },
                Ok(None) => {
                    match child.wait() {
                        Ok(status) => {
                            sender.report_status(pid, &status);
                            if status.success() {
                                Ok(())
                            } else {
                                bail!(ErrorKind::RuntimeError("process exited with non zero status"));
                            }
                        },
                        Err(_) => {
                            // TODO: report something
                            bail!(ErrorKind::RuntimeError("command status retrival failed"))
                        },
                    }
                },
                Err(error) => {
                    // TODO: report something
                    bail!(ErrorKind::RuntimeError("command execution failed"))
                },
            }
        },
        Err(err) => {
            sender.report_failed(&cmd, format!("{}", err));
            bail!(ErrorKind::RuntimeError("command not found"))
        },
    }
}

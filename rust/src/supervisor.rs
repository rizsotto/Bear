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

use chrono;

use {ErrorKind, Result, ResultExt};
use event::*;

pub struct Supervisor {
    child: process::Child,
    parent: ProcessId,
    cmd: Vec<String>,
    cwd: path::PathBuf,
    running: bool,
}

impl Supervisor {
    pub fn new(cmd: &[String], parent: ProcessId) -> Result<Supervisor> {
        let cwd = env::current_dir()
            .chain_err(|| "unable to get current working directory")?;
        let child = process::Command::new(&cmd[0]).args(&cmd[1..]).spawn()
            .chain_err(|| format!("unable to execute process: {:?}", cmd[0]))?;

        debug!("process was started: {:?}", child.id());
        Ok(Supervisor { child, parent, cmd: cmd.to_vec(), cwd, running: true })
    }

    pub fn wait<F>(&mut self, listener: &mut F) -> Result<()>
        where F: FnMut(Event) -> Result<()> {

        self.report(self.created(), listener);
        match self.child.wait() {
            Ok(status) => {
                debug!("process was stopped: {:?}", self.child.id());
                self.report(self.terminated(status), listener);
                self.running = false;
            }
            Err(_) => {
                warn!("process was not running: {:?}", self.child.id());
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.running {
            match self.child.kill() {
                Ok(()) => {
                    debug!("Process kill successful: {:?}", self.child.id());
                }
                Err(_) => {
                    debug!("Process kill failed: {:?}", self.child.id());
                }
            }
        } else {
            debug!("Process kill not needed: {:?}", self.child.id());
        }
    }

    fn created(&self) -> Event {
        let message = ProcessCreated {
            pid: self.child.id(),
            ppid: self.parent,
            cwd: self.cwd.clone(),
            cmd: self.cmd.clone(),
        };
        Event::Created(message, chrono::Utc::now())
    }

    fn terminated(&self, status: process::ExitStatus) -> Event {
        let pid = self.child.id();
        match status.code() {
            Some(code) => {
                let message = ProcessTerminatedNormally { pid, code };
                Event::TerminatedNormally(message, chrono::Utc::now())
            }
            None => {
                let message = ProcessTerminatedAbnormally { pid, signal: -1 };
                Event::TerminatedAbnormally(message, chrono::Utc::now())
            }
        }
    }

    fn report<F>(&self, event: Event, listener: &mut F)
        where F: FnMut(Event) -> Result<()> {
        match listener(event) {
            Ok(_) => debug!("Event sent."),
            Err(error) => debug!("Event sending failed. {:?}", error),
        }
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop()
    }
}

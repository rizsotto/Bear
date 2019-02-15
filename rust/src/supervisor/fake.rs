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

use crate::{ErrorKind, Result, ResultExt};
use crate::compilation::compiler::*;
use crate::compilation::execution::*;
use crate::event::*;

pub struct Supervisor<F>
    where F: FnMut(Event) -> Result<()>
{
    sink: F,
}

impl<F> Supervisor<F>
    where F: FnMut(Event) -> Result<()>
{
    pub fn new(sink: F) -> Supervisor<F> {
        Supervisor { sink }
    }

    pub fn run(&mut self, cmd: &[String]) -> Result<ExitCode> {
        let cwd = env::current_dir()
            .chain_err(|| "unable to get current working directory")?;
        let pid = process::id();

        let event = Event::Created(
            ProcessCreated {
                pid,
                ppid: get_parent_pid(),
                cwd: cwd.clone(),
                cmd: cmd.to_vec(), },
            chrono::Utc::now());
        self.report(event);

        let event = match create_output_file() {
            Ok(_) => {
                let message = ProcessTerminated { pid, code: 0 };
                Event::TerminatedNormally(message, chrono::Utc::now())
            }
            Err(_) => {
                let message = ProcessTerminated { pid, code: -1 };
                Event::TerminatedNormally(message, chrono::Utc::now())
            }
        };
        self.report(event);

        Ok(0)
    }

    fn report(&mut self, event: Event) {
        match (self.sink)(event) {
            Ok(_) => debug!("Event sent."),
            Err(error) => debug!("Event sending failed. {:?}", error),
        }
    }
}

#[cfg(not(unix))]
pub fn get_parent_pid() -> ProcessId {
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

#[cfg(unix)]
pub fn get_parent_pid() -> ProcessId {
    std::os::unix::process::parent_id() as ProcessId
}

fn create_output_file() -> Result<()> {
    unimplemented!()
}
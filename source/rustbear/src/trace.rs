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
use std::io;
use std::fs::{OpenOptions, File};
use std::path::{Path, PathBuf};
use libc;
use serde_json;

use Result;

#[derive(Serialize, Deserialize)]
pub struct Trace {
    pid: libc::pid_t,
    cwd: PathBuf,
    cmd: Vec<String>
}

impl Trace {
    /// Create an Trace report object from the given arguments.
    /// Capture the current process id and working directory.
    pub fn create(args: &Vec<String>) -> Result<Trace> {
        let pid: libc::pid_t = unsafe { libc::getpid() };
        let cwd = env::current_dir()?;
        Ok(Trace { pid: pid, cwd: cwd, cmd: args.clone() })
    }

    pub fn get_pid(&self) -> &libc::pid_t {
        &self.pid
    }

    pub fn get_cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn get_cmd(&self) -> &[String] {
        &self.cmd
    }

    /// Write a single trace entry into the given target.
    pub fn write(target: &mut io::Write, value: &Trace) -> Result<()> {
        let result = serde_json::to_writer(target, value)?;
        Ok(result)
    }

    /// Read a single trace file content from given source.
    pub fn read(source: &mut io::Read) -> Result<Trace> {
        let result = serde_json::from_reader(source)?;
        Ok(result)
    }
}

pub fn create_writer(path: &Path) -> Result<File> {
    let file_name = path.join("random");  // todo: generate random filename
    let file = OpenOptions::new().write(true).open(file_name)?;
    Ok(file)
}

pub fn create_reader(path: &Path) -> Result<File> {
    let file = OpenOptions::new().read(true).open(path)?;
    Ok(file)
}

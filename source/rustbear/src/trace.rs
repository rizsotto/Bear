// Copyright (c) 2017 László Nagy
//
// Licensed under the MIT license <LICENSE or http://opensource.org/licenses/MIT>.
// This file may not be copied, modified, or distributed except according to those terms.

use std::env;
use std::io;
use std::fs::{OpenOptions, File};
use std::path::Path;
use std::ffi::OsString;
use libc;
use serde_json;

use Result;

#[derive(Serialize, Deserialize)]
pub struct Trace {
    pid: libc::pid_t,
    cwd: OsString,
    cmd: Vec<OsString>
}

impl Trace {
    /// Create an Trace report object from the given arguments.
    /// Capture the current process id and working directory.
    pub fn create(args: &Vec<OsString>) -> Result<Trace> {
        let pid: libc::pid_t = unsafe { libc::getpid() };
        let cwd = env::current_dir()?;
        Ok(Trace { pid: pid, cwd: cwd.into_os_string(), cmd: args.clone() })
    }

    pub fn get_pid(&self) -> &libc::pid_t {
        &self.pid
    }

    pub fn get_cwd(&self) -> &OsString {
        &self.cwd
    }

    pub fn get_cmd(&self) -> &[OsString] {
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

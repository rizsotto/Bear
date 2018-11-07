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

use std::path;
use std::process;
use std::str;
use shellwords;

use trace;
use Result;
use Error;

pub enum CompilerPass {
    Preprocessor,
    Compilation,
    Assembly,
    Linking
}

pub struct CompilerExecution {
    compiler: path::PathBuf,
    phase: CompilerPass,
    flags: Vec<String>,
    inputs: Vec<path::PathBuf>,
    output: Option<path::PathBuf>
}

impl CompilerExecution {

    pub fn from(_trace: &trace::Trace) -> Option<CompilerExecution> {
        unimplemented!()
    }

}

/// Takes a command string and returns as a list.
fn shell_split(string: &str) -> Result<Vec<String>> {
    match shellwords::split(string) {
        Ok(value) => Ok(value),
        _ => Err(Error::RuntimeError("Can't parse shell command"))
    }
}

/// Provide information on how the underlying compiler would have been
/// invoked without the MPI compiler wrapper.
fn get_mpi_call(wrapper: &String) -> Result<Vec<String>> {
    fn run_mpi_wrapper(wrapper: &String, flag: &str) -> Result<Vec<String>> {
        let child = process::Command::new(wrapper)
            .arg(flag)
            .stdout(process::Stdio::piped())
            .spawn()?;
        let output = child.wait_with_output()?;
        // Take the stdout if the process was successful.
        if output.status.success() {
            let string = str::from_utf8(output.stdout.as_slice())?;
            // Take only the first line.
            let lines: Vec<&str> = string.lines().collect();
            // And treat as it would be a shell command.
            match lines.first() {
                Some(first_line) => shell_split(first_line),
                _ => Err(Error::RuntimeError("Empty output of wrapper"))
            }
        } else {
            Err(Error::RuntimeError("Process failed."))
        }
    }

    // Try both flags with the wrapper and return the first successful result.
    ["--show", "--showme"].iter()
        .map(|&query_flatg| run_mpi_wrapper(wrapper, &query_flatg))
        .find(Result::is_ok)
        .unwrap_or(Err(Error::RuntimeError("Could not determinate MPI flags.")))
}

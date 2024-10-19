/*  Copyright (C) 2012-2024 by László Nagy
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

extern crate core;

use std::path::PathBuf;
use anyhow::Result;
use intercept::reporter::{Reporter, TcpReporter};

fn main() -> Result<()> {
    // Find out what is the executable name the execution was started with
    let executable = std::env::args().next().unwrap();
    // Read the PATH variable and find the next executable with the same name
    let real_executable = std::env::var("PATH")?
        .split(':')
        .map(|dir| std::path::Path::new(dir).join(&executable))
        .filter(|path| path.exists())
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Cannot find the real executable"))?;
    // TODO: ^ This is a very naive way to find the real executable.
    //        Make sure we don't call ourselves.

    // Report the execution with the real executable
    report_execution(&real_executable);

    // Execute the real executable with the same arguments
    let status = std::process::Command::new(real_executable)
        .args(std::env::args().skip(1))
        .status()?;
    // Return the status code
    std::process::exit(status.code().unwrap_or(1));
}

// TODO: Current error handling is very basic, it just panics on any error.
//      More sophisticated error handling can be: logging the error and return.
fn report_execution(path_buf: &PathBuf) {
    // Get the reporter address from the environment
    let reporter_address = std::env::var(INTERCEPT_REPORTER_ADDRESS)
        .expect(format!("${} is not set", INTERCEPT_REPORTER_ADDRESS).as_str());
    // Create a new reporter
    let reporter = TcpReporter::new(reporter_address)
        .expect("Cannot create reporter");

    // Report the execution
    let execution = intercept::Event {
        pid: intercept::ProcessId(std::process::id() as u32),
        execution: intercept::Execution {
            executable: path_buf.clone(),
            arguments: std::env::args().collect(),
            working_dir: std::env::current_dir().expect("Cannot get current directory"),
            environment: std::env::vars().collect(),
        },
    };
    reporter.report(execution)
        .expect("Cannot report execution");
}

// declare a const string for the INTERCEPT_REPORTER_ADDRESS environment name
const INTERCEPT_REPORTER_ADDRESS: &str = "INTERCEPT_REPORTER_ADDRESS";
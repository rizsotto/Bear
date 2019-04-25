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

#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use env_logger;

use std::env;
use std::process;
use std::error::Error;

use ear::Result;
use ear::intercept::report;
use ear::intercept::ExitCode;

fn main() {
    match run() {
        Ok(code) => {
            process::exit(code);
        },
        Err(e) => {
            eprintln!("error: {}", e);
            if let Some(source) = e.source() {
                eprintln!("caused by: {}", source);
            }
            process::exit(1);
        },
    }
}

fn run() -> Result<ExitCode> {
    env_logger::init();
    info!("cxx - {} {}", crate_name!(), crate_version!());

    let args: Vec<String> = env::args().collect();
    debug!("invocation: {:?}", &args);

    let code = report::c_compiler(args.as_ref())?;
    Ok(code)
}

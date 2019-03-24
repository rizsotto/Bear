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
extern crate env_logger;
extern crate error_chain;
extern crate ear;
#[macro_use]
extern crate log;

#[cfg(unix)]
extern crate nix;

use std::env;
use std::path;
use std::process;

use ear::Result;
use ear::environment::{KEY_CXX, KEY_DESTINATION};
use ear::intercept::ExitCode;
use ear::intercept::supervisor::Supervisor;
use ear::intercept::protocol::sender::Protocol;

fn main() {
    match run() {
        Ok(code) => {
            process::exit(code);
        },
        Err(ref e) => {
            eprintln!("error: {}", e);

            for e in e.iter().skip(1) {
                eprintln!("caused by: {}", e);
            }

            // The backtrace is not always generated. Try to run this with `RUST_BACKTRACE=1`.
            if let Some(backtrace) = e.backtrace() {
                eprintln!("backtrace: {:?}", backtrace);
            }
            process::exit(1);
        },
    }
}

fn run() -> Result<ExitCode> {
    drop(env_logger::init());
    info!("cxx - {} {}", crate_name!(), crate_version!());

    let mut args: Vec<String> = env::args().collect();
    debug!("invocation: {:?}", &args);

    let target = env::var(KEY_DESTINATION)?;
    let mut protocol = Protocol::new(path::Path::new(target.as_str()))?;

    let mut supervisor = Supervisor::new(|event| protocol.send(event));

    match env::var(KEY_CXX) {
        Ok(wrapper) => {
            args[0] = wrapper;
            supervisor.run(&args[..])
        },
        Err(_) => {
            supervisor.fake(&args[..])
        },
    }
}

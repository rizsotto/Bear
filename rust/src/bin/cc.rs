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
extern crate intercept;
#[macro_use]
extern crate log;

use std::env;

use intercept::Result;
use intercept::event::*;
use intercept::supervisor::Supervisor;
use intercept::protocol::Protocol;

fn main() {
    if let Err(ref e) = run() {
        eprintln!("error: {}", e);

        for e in e.iter().skip(1) {
            eprintln!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            eprintln!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    drop(env_logger::init());
    info!("{} {}", crate_name!(), crate_version!());

    let args: Vec<String> = env::args().collect();
    debug!("invocation: {:?}", &args);

    let mut protocol = Protocol::new()?;
    let mut child = Supervisor::new(&args[1..], get_parent_pid())?;

    child.wait(&mut |event: Event| protocol.send(event))
}

#[cfg(unix)]
fn get_parent_pid() -> ProcessId {
    let ppid: libc::pid_t = unsafe { libc::getppid() };
    ppid as ProcessId
}

#[cfg(not(unix))]
fn get_parent_pid() -> ProcessId {
    match env::var("INTERCEPT_PPID") {
        Ok(value) => {
            let ppid: ProcessId = value.parse().unwrap();
            ppid
        },
        _ => 0,
    }
}

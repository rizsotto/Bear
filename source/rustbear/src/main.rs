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

extern crate env_logger;
extern crate intercept;
#[macro_use]
extern crate log;

use intercept::cli;
use intercept::config;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    drop(env_logger::init());
    debug!("Invocation: {:?}", &args);

    match cli::Config::parse(&args) {
        Ok(cli) => do_things(cli),
        Err(error) => eprintln!("{:?}", error),
    }
}

fn do_things(cli: cli::Config) {
    let build = cli.build;
    let mut command = process::Command::new(&build[0]);
    command.args(&build[1..]);

    match command.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status_code) => process::exit(status_code.code().unwrap_or(130)), // 128 + signal
            Err(_) => process::exit(64),                                         // not used yet
        },
        Err(_) => process::exit(127), // command not found
    }
}

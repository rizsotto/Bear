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

extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate clap;
extern crate env_logger;
extern crate intercept;

use std::process;
use clap::{App, Arg};

fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(Arg::with_name("verbose")
            .long("verbose")
            .short("v")
            .multiple(true)
            .help("Sets the level of verbosity"))
        .arg(Arg::with_name("output")
            .long("output")
            .short("o")
            .takes_value(true)
            .value_name("file")
            .default_value("compile_commands.json")
            .help("The compilation database file"))
        .arg(Arg::with_name("build")
            .multiple(true)
            .allow_hyphen_values(true)
            .required(true)
            .help("The build command to intercept"))
        .get_matches();

    let build: Vec<_> = matches.values_of("build").unwrap().collect();
    let mut command = process::Command::new(build[0]);
    command.args(&build[1..]);

    match command.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status_code) => process::exit(status_code.code().unwrap_or(130)), // 128 + signal
            Err(_) => process::exit(64), // not used yet
        },
        Err(_) => process::exit(127), // command not found
    }
}

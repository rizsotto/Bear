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
use std::path;
use std::process;

use intercept::{Result, ResultExt};
use intercept::database::file::JsonCompilationDatabase;
use intercept::database::builder::Builder;
use intercept::database::config::Config;
use intercept::environment::KEY_DESTINATION;
use intercept::event::ExitCode;
use intercept::supervisor::Supervisor;
use intercept::protocol;


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

            ::std::process::exit(1);
        },
    }
}

const OUTPUT_FLAG: &str = "output";
const BUILD_FLAG: &str = "build";

fn run() -> Result<ExitCode> {
    drop(env_logger::init());
    info!("bear - {} {}", crate_name!(), crate_version!());

    let args: Vec<String> = env::args().collect();
    debug!("invocation: {:?}", &args);

    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            clap::Arg::with_name(OUTPUT_FLAG)
                .long(OUTPUT_FLAG)
                .short("o")
                .takes_value(true)
                .value_name("file")
                .default_value("compile_commands.json")
                .help("The compilation database file"),
        ).arg(
            clap::Arg::with_name(BUILD_FLAG)
                .multiple(true)
                .allow_hyphen_values(true)
                .required(true)
                .help("The build command to intercept"),
        ).get_matches_from(args);

    let output = path::PathBuf::from(matches.value_of(OUTPUT_FLAG).unwrap());
    debug!("output file: {:?}", output);

    let command: Vec<_> = matches
        .values_of(BUILD_FLAG)
        .unwrap()
        .map(|s| s.to_string())
        .collect();
    debug!("command to run: {:?}", command);

    let config = Config::default();
    let target =
        JsonCompilationDatabase::new(
            path::Path::new("./compile_commands.json"));
    let builder = Builder::new(&config, &target);

    intercept_build(&builder, command.as_ref())
}

fn intercept_build(builder: &Builder, command: &[String]) -> Result<ExitCode> {
    let collector = protocol::collector::Protocol::new()
        .chain_err(|| "Failed to set up event collection.")?;

    let exit = run_build(command, collector.path())
        .chain_err(|| "Failed to run the build.")?;

    builder.build(collector.events())
        .chain_err(|| "Failed to write output.")?;

    Ok(exit)
}

fn run_build(command: &[String], destination: &path::Path) -> Result<ExitCode> {
    env::set_var(KEY_DESTINATION, destination);

    let mut sender = protocol::sender::Protocol::new(destination)?;
    let mut build = Supervisor::new(|event| sender.send(event));
    let exit = build.run(command)?;
    info!("Build finished with status code: {}", exit);
    Ok(exit)
}

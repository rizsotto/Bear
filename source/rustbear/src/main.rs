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
#[macro_use]
extern crate error_chain;
extern crate intercept;
#[macro_use]
extern crate log;

use intercept::config::Config;
use intercept::{Error, Result};
use std::env;
use std::path;
use std::process;

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

fn run() -> intercept::Result<()> {
    const CONFIG_FLAG: &str = "config";
    const OUTPUT_FLAG: &str = "output";
    const BUILD_FLAG: &str = "build";

    drop(env_logger::init());

    let args: Vec<String> = env::args().collect();
    debug!("invocation: {:?}", &args);

    let default_config_path = default_config_path();

    let matches = clap::App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            clap::Arg::with_name(CONFIG_FLAG)
                .long(CONFIG_FLAG)
                .short("c")
                .takes_value(true)
                .value_name("file")
                .default_value(default_config_path.as_ref())
                .help("The configuration file"),
        ).arg(
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

    let build: Vec<_> = matches
        .values_of(BUILD_FLAG)
        .unwrap()
        .map(|s| s.to_string())
        .collect();
    debug!("command to run: {:?}", build);

    let config_string = matches.value_of(CONFIG_FLAG).unwrap().to_string();
    let config = Config::read(config_string.as_ref())
        .map_err(|e| Error::with_chain(e, "could not read config"))?;
    debug!("config: {:?}", config);

    intercept_build(build.as_ref())
}

fn default_config_path() -> String {
    let default_config_path = match dirs::config_dir() {
        Some(dir) => dir.join("bear.conf"),
        None => path::PathBuf::from("bear.conf"),
    };
    default_config_path.into_os_string().into_string().unwrap()
}

fn intercept_build(build: &[String]) -> Result<()> {
    let mut command = process::Command::new(&build[0]);
    match command.args(&build[1..]).spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status_code) => {
                if 0 == status_code.code().unwrap_or(130) {
                    Ok(())
                } else {
                    bail!(intercept::ErrorKind::RuntimeError(""));
                }
            }
            Err(_) => bail!(intercept::ErrorKind::RuntimeError("")),
        },
        Err(_) => bail!(intercept::ErrorKind::RuntimeError("command not found")),
    }
}

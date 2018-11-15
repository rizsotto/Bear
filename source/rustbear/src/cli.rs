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

use clap;
use std::path;

use Error;
use Result;

const VERBOSE_FLAG: &str = "verbose";
const CONFIG_FLAG: &str = "config";
const OUTPUT_FLAG: &str = "output";
const BUILD_FLAG: &str = "build";

const CONFIG_FLAG_DEFAULT: &str = "~/.config/bear.conf";
const OUTPUT_FLAG_DEFAULT: &str = "compile_commands.json";

#[derive(Debug)]
pub struct Config {
    pub verbose: usize,
    pub config: path::PathBuf,
    pub output: path::PathBuf,
    pub build: Vec<String>,
}

impl Config {
    pub fn parse(args: &[String]) -> Result<Config> {
        let matches = clap::App::new(crate_name!())
            .version(crate_version!())
            .about(crate_description!())
            .arg(
                clap::Arg::with_name(VERBOSE_FLAG)
                    .long(VERBOSE_FLAG)
                    .short("v")
                    .multiple(true)
                    .help("Sets the level of verbosity"),
            ).arg(
                clap::Arg::with_name(CONFIG_FLAG)
                    .long(CONFIG_FLAG)
                    .short("c")
                    .takes_value(true)
                    .value_name("file")
                    .default_value(CONFIG_FLAG_DEFAULT)
                    .help("The configuration file"),
            ).arg(
                clap::Arg::with_name(OUTPUT_FLAG)
                    .long(OUTPUT_FLAG)
                    .short("o")
                    .takes_value(true)
                    .value_name("file")
                    .default_value(OUTPUT_FLAG_DEFAULT)
                    .help("The compilation database file"),
            ).arg(
                clap::Arg::with_name(BUILD_FLAG)
                    .multiple(true)
                    .allow_hyphen_values(true)
                    .required(true)
                    .help("The build command to intercept"),
            ).get_matches_from(args);

        let result = Config {
            verbose: matches.values_of(VERBOSE_FLAG).iter().count(),
            config: path::PathBuf::from(
                matches.value_of(CONFIG_FLAG).unwrap_or(CONFIG_FLAG_DEFAULT),
            ),
            output: path::PathBuf::from(
                matches.value_of(OUTPUT_FLAG).unwrap_or(OUTPUT_FLAG_DEFAULT),
            ),
            build: matches
                .values_of(BUILD_FLAG)
                .unwrap()
                .map(|s| s.to_string())
                .collect(),
        };
        Config::validate(result)
    }

    fn validate(self) -> Result<Self> {
        if !self.config.is_file() {
            Err(Error::Config(config_file_not_found(self.config.as_path())))
        } else {
            Ok(self)
        }
    }
}

fn config_file_not_found(file: &path::Path) -> String {
    let message = "Config file not found: ".to_string();
    message + file.to_str().unwrap()
}

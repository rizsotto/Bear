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

use ErrorKind;
use Result;

const CONFIG_FLAG: &str = "config";
const OUTPUT_FLAG: &str = "output";
const BUILD_FLAG: &str = "build";

const CONFIG_FLAG_DEFAULT: &str = "~/.config/bear.conf";
const OUTPUT_FLAG_DEFAULT: &str = "compile_commands.json";

#[derive(Debug)]
pub struct Arguments {
    pub config: path::PathBuf,
    pub output: path::PathBuf,
    pub build: Vec<String>,
}

impl Arguments {
    pub fn parse(args: &[String]) -> Self {
        let matches = clap::App::new(crate_name!())
            .version(crate_version!())
            .about(crate_description!())
            .arg(
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

        Self {
            config: path::PathBuf::from(matches.value_of(CONFIG_FLAG).unwrap()),
            output: path::PathBuf::from(matches.value_of(OUTPUT_FLAG).unwrap()),
            build: matches
                .values_of(BUILD_FLAG)
                .unwrap()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<&Self> {
        if !self.config.is_file() {
            let message = self.config.to_str().map(|utf| utf.to_string()).unwrap();
            Err(ErrorKind::ConfigFileNotFound(message).into())
        } else {
            Ok(self)
        }
    }
}

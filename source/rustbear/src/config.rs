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
use std::fs;
use std::io;
use std::path;

use ErrorKind;
use Result;

#[derive(Debug)]
pub struct Arguments {
    config: String,
    config_from_stdin: bool,
    pub output: path::PathBuf,
    pub build: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub intercept: Intercept,
    pub output: Output,
    pub sources: Sources,
    pub compilers: Compilers,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Intercept {
    Preload {
        library: path::PathBuf,
        process: path::PathBuf,
    },
    Wrapper {
        cc_wrapper: CompilerWrapper,
        cxx_wrapper: CompilerWrapper,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompilerWrapper {
    pub wrapper: path::PathBuf,
    pub compiler: path::PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub relative_to: path::PathBuf,
    pub command_format: CommandFormat,
    pub headers: bool,
    pub output: bool,
    pub append: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandFormat {
    Array,
    String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sources {
    pub extensions: Vec<String>,
    pub paths: Vec<path::PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Compilers {
    pub languages: CompilersLanguages,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompilersLanguages {
    pub c_compilers: Vec<String>,
    pub cxx_compilers: Vec<String>,
    pub mpi_compilers: Vec<String>,
    pub compiler_wrappers: Vec<String>,
}

const CONFIG_FLAG: &str = "config";
const OUTPUT_FLAG: &str = "output";
const BUILD_FLAG: &str = "build";

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
                    .default_value("~/.config/bear.conf")
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

        Self {
            config: matches.value_of(CONFIG_FLAG).unwrap().to_string(),
            config_from_stdin: matches.value_of(CONFIG_FLAG) == Some("-"),
            output: path::PathBuf::from(matches.value_of(OUTPUT_FLAG).unwrap()),
            build: matches
                .values_of(BUILD_FLAG)
                .unwrap()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    pub fn read_config(&self) -> Result<Config> {
        if self.config_from_stdin {
            let reader = io::stdin();
            let result = serde_yaml::from_reader(reader.lock())?;
            Ok(result)
        } else {
            let filename = path::PathBuf::from(&self.config);
            if filename.is_file() {
                let reader = fs::File::open(filename.as_path())?;
                let result = serde_yaml::from_reader(reader)?;
                Ok(result)
            } else {
                let message = self.config.clone();
                Err(ErrorKind::ConfigFileNotFound(message).into())
            }
        }
    }
}

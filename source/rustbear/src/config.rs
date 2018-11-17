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

use serde_yaml;
use std::fs;
use std::io;
use std::path;
use {ErrorKind, Result};

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

impl Config {
    pub fn read(config_string: &str) -> Result<Self> {
        if config_string == "-" {
            debug!("reading stdin for config");
            let reader = io::stdin();
            let result = serde_yaml::from_reader(reader.lock())?;
            Ok(result)
        } else {
            debug!("reading file for config '{}'", config_string);
            let filename = path::PathBuf::from(&config_string);
            if filename.is_file() {
                let reader = fs::File::open(filename.as_path())?;
                let result = serde_yaml::from_reader(reader)?;
                Ok(result)
            } else {
                let message = config_string.to_string();
                bail!(ErrorKind::ConfigFileNotFound(message))
            }
        }
    }
}

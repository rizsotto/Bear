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

use std::env;
use std::io;
use std::ffi::OsString;
use serde_json;

use Result;
use Error;


#[derive(Serialize, Deserialize)]
pub struct Parameters {
    cc: OsString,
    cxx: OsString,
    target: OsString,
}

impl Parameters {
    pub fn new(cc: &str, cxx: &str, target: &str) -> Parameters {
        Parameters {
            cc: OsString::from(cc),
            cxx: OsString::from(cxx),
            target: OsString::from(target)
        }
    }

    pub fn get_cc(&self) -> &OsString {
        &self.cc
    }

    pub fn get_cxx(&self) -> &OsString {
        &self.cxx
    }

    pub fn get_target(&self) -> &OsString {
        &self.target
    }

    /// Serialize the parameters into a string.
    pub fn write(&self) -> Result<OsString> {
        let result = serde_json::to_string(self)?;
        Ok(OsString::from(result))
    }

    /// Deserialize the parameters from a string.
    pub fn read(source: &OsString) -> Result<Parameters> {
        match source.to_str() {
            Some(string) => {
                let result = serde_json::from_str(string)?;
                Ok(result)
            }
            None => Err(Error::Io(io::Error::from(io::ErrorKind::InvalidInput)))
        }
    }
}


const ENV_KEY: &'static str = "__BEAR";

/// Create a key-value pair from parameters to store that in environment.
pub fn to_env(parameters: &Parameters) -> Result<(OsString, OsString)> {
    let key: OsString = OsString::from(ENV_KEY);
    parameters.write().map(|value| (key, value))
}

/// Read parameters from environment variables.
pub fn from_env() -> Result<Parameters> {
    let value = env::var(ENV_KEY)?;
    Parameters::read(&OsString::from(&value))
}

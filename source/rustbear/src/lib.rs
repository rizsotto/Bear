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

extern crate libc;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
//#[macro_use] extern crate log;
extern crate tempdir;
extern crate shellwords;
#[macro_use] extern crate lazy_static;
extern crate regex;


pub mod trace;
pub mod database;
pub mod compilation;


use std::result;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Env(std::env::VarError),
    Json(serde_json::Error),
    String(std::str::Utf8Error),
    RuntimeError(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Error {
        Error::Env(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Error {
        Error::String(err)
    }
}

type Result<T> = result::Result<T, Error>;

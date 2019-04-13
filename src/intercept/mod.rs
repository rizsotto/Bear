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

pub mod event;
pub mod report;
pub mod supervisor;
pub mod inner;

pub use self::event::*;
pub use self::report::*;

use std::fmt;
use std::error;

#[derive(Debug)]
pub enum Error {
    Configuration {
        key: &'static str,
    },
    Execution {
        program: String,
        #[cfg(unix)]
        cause: ::nix::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Configuration { key, .. } => {
                write!(f, "Could not find {} in the current environment.", key)
            },
            Error::Execution { program, .. } => {
                write!(f, "Failed to execute: {}", program)
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            #[cfg(unix)]
            Error::Execution { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

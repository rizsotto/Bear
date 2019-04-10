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

extern crate chrono;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate regex;
#[macro_use]
extern crate scopeguard;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate shellwords;
extern crate tempfile;

#[cfg(unix)]
extern crate nix;

pub mod intercept;
pub mod semantic;
pub mod output;
pub mod command;

mod error {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            Env(::std::env::VarError);
            Num(::std::num::ParseIntError);
            String(::std::str::Utf8Error);
            Json(::serde_json::Error);
            Nix(::nix::Error) #[cfg(unix)];
        }

        errors {
            CompilationError(msg: &'static str) {
                description("compilation error"),
                display("compilation error: '{}'", msg),
            }

            RuntimeError(msg: &'static str) {
                description("runtime error"),
                display("runtime error: '{}'", msg),
            }
        }
    }
}

pub use error::{Error, ErrorKind, Result, ResultExt};

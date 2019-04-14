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
pub use self::supervisor::*;

mod error {
    error_chain! {
        links {
            Semantic(crate::semantic::Error, crate::semantic::ErrorKind);
        }
        foreign_links {
            Io(::std::io::Error);
            Env(::std::env::VarError);
            Num(::std::num::ParseIntError);
            Nix(::nix::Error) #[cfg(unix)];
            Json(::serde_json::Error);
        }
    }
}

pub use self::error::{Error, ErrorKind, Result, ResultExt};

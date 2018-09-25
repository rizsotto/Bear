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

use std::ffi;

use trace::Trace;
use database::Entry;
use Result;

pub enum CompilerPass {
    Preprocessor,
    Compilation,
    Assembly,
    Linking
}

pub struct Compilation {
    compiler: ffi::OsString,
    phase: CompilerPass,
    flags: Vec<ffi::OsString>,
    source: ffi::OsString,
    output: Option<ffi::OsString>,
    cwd: ffi::OsString,
}

impl Compilation {
    pub fn from_trace(trace: Trace) -> Result<Compilation> {
        unimplemented!()
    }

    pub fn from_db_entry(entry: Entry) -> Result<Compilation> {
        unimplemented!()
    }

    pub fn to_db_entry(&self) -> Result<Entry> {
        unimplemented!()
    }

    pub fn to_relative(&self, to: ffi::OsString) -> Result<Compilation> {
        unimplemented!()
    }

    pub fn to_absolute(&self, to: ffi::OsString) -> Result<Compilation> {
        unimplemented!()
    }
}
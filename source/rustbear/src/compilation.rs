// Copyright (c) 2017 László Nagy
//
// Licensed under the MIT license <LICENSE or http://opensource.org/licenses/MIT>.
// This file may not be copied, modified, or distributed except according to those terms.

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
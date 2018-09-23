// Copyright (c) 2017 László Nagy
//
// Licensed under the MIT license <LICENSE or http://opensource.org/licenses/MIT>.
// This file may not be copied, modified, or distributed except according to those terms.

use std::io;
use std::ffi;
use serde_json;

use Result;

#[derive(Serialize, Deserialize)]
pub struct Entry {
    directory: ffi::OsString,
    file: ffi::OsString,
    output: Option<ffi::OsString>,
    #[serde(skip_serializing)]
    command: Option<ffi::OsString>,
    arguments: Vec<ffi::OsString>
}

type Entries = Vec<Entry>;


impl Entry {
    pub fn new(directory: ffi::OsString,
               file: ffi::OsString,
               output: Option<ffi::OsString>,
               arguments: Vec<ffi::OsString>) -> Entry {
        Entry {
            directory: directory,
            file: file,
            output: output,
            command: None,
            arguments: arguments
        }
    }

    pub fn get_directory(&self) -> &ffi::OsString {
        &self.directory
    }

    pub fn get_file(&self) -> &ffi::OsString {
        &self.file
    }

    pub fn get_output(&self) -> &Option<ffi::OsString> {
        &self.output
    }

    pub fn get_arguments(&self) -> &[ffi::OsString] {
        &self.arguments
    }

    fn get_command(&self) -> &Option<ffi::OsString> {
        &self.command
    }
}


pub fn read(source: &mut io::Read) -> Result<Entries> {
    let result: Entries = serde_json::from_reader(source)?;
    // todo: transform the entries into one which has arguments.
    Ok(result)
}

pub fn write(target: &mut io::Write, value: &Entries) -> Result<()> {
    let result = serde_json::to_writer(target, value)?;
    Ok(result)
}

fn parse(command: ffi::OsString) -> Result<Vec<ffi::OsString>> {
    unimplemented!()
}

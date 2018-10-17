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

use std::io;
use std::path;
use serde_json;

use Result;

pub struct Entry {
    directory: path::PathBuf,
    file: path::PathBuf,
    command: Vec<String>,
    output: Option<path::PathBuf>
}

type Entries = Vec<Entry>;


impl Entry {
    pub fn new(directory: path::PathBuf,
               file: path::PathBuf,
               output: Option<path::PathBuf>,
               arguments: Vec<String>) -> Entry {
        Entry {
            directory: directory,
            file: file,
            command: arguments,
            output: output
        }
    }

    pub fn get_directory(&self) -> &path::Path {
        &self.directory
    }

    pub fn get_file(&self) -> &path::Path {
        &self.file
    }

    pub fn get_command(&self) -> &[String] {
        &self.command
    }

//    pub fn get_output(&self) -> &Option<Path> {
//        &self.output
//    }
}

#[derive(Serialize, Deserialize)]
pub struct IoEntry {
    directory: path::PathBuf,
    file: path::PathBuf,
    command: Vec<String>,
    output: Option<path::PathBuf>
}

type IoEntries = Vec<IoEntry>;


impl From<Entry> for IoEntry {
    fn from(_: Entry) -> Self {
        unimplemented!()
    }
}

impl Into<Entry> for IoEntry {
    fn into(self) -> Entry {
        unimplemented!()
    }
}

pub fn read(source: &mut io::Read) -> Result<Entries> {
    let io_result: IoEntries = serde_json::from_reader(source)?;
    let result: Entries = io_result.into_iter().map(IoEntry::into).collect();
    Ok(result)
}

pub fn write(target: &mut io::Write, value: &Entries) -> Result<()> {
//    let io_value: IoEntries = value.into_iter().map(IoEntry::from).collect();
//    let result = serde_json::to_writer(target, &io_value)?;
//    Ok(result)
    unimplemented!()
}

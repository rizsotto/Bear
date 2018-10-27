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

use std::collections;
use std::io;
use std::iter::FromIterator;
use std::path;
use serde_json;
use Result;


#[derive(Hash, Serialize, Deserialize)]
pub struct Entry {
    directory: path::PathBuf,
    file: path::PathBuf,
    command: Vec<String>,
    output: Option<path::PathBuf>
}

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

    pub fn get_output(&self) -> &Option<path::PathBuf> {
        &self.output
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.directory == other.directory &&
        self.file == other.file &&
        self.command == other.command
    }
}

impl Eq for Entry {}


pub struct Database {
    entries: collections::HashSet<Entry>
}

impl Database {
    pub fn new() -> Database {
        Database { entries: collections::HashSet::new() }
    }

    pub fn load(&mut self, source: &mut io::Read) -> Result<()> {
        let entries: Vec<Entry> = serde_json::from_reader(source)?;
        let result = self.add_entries(entries);
        Ok(result)
    }

    pub fn save(&self, target: &mut io::Write) -> Result<()> {
        let values = Vec::from_iter(self.entries.iter());
        let result = serde_json::to_writer(target, &values)?;
        Ok(result)
    }

    pub fn add_entry(&mut self, entry: Entry) -> () {
        self.entries.insert(entry);
    }

    pub fn add_entries(&mut self, entries: Vec<Entry>) -> () {
        let fresh: collections::HashSet<Entry> = collections::HashSet::from_iter(entries);
        self.entries.union(&fresh);
    }
}
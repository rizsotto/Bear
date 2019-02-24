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
use std::path;

use Result;


/// Represents a generic entry of the compilation database.
#[derive(Hash)]
pub struct Entry {
    pub directory: path::PathBuf,
    pub file: path::PathBuf,
    pub command: Vec<String>,
    pub output: Option<path::PathBuf>,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Entry) -> bool {
        self.directory == other.directory
            && self.file == other.file
            && self.command == other.command
    }
}

impl Eq for Entry {
}

type Entries = collections::HashSet<Entry>;


/// Represents the expected format of the JSON compilation database.
pub struct DatabaseFormat {
    command_as_array: bool,

    // Other attributes might be:
    // - output present or not
    // - paths are relative or absolute
}

impl DatabaseFormat {
    pub fn new() -> Self {
        DatabaseFormat {
            command_as_array: true,
        }
    }

    pub fn command_as_array(&mut self, value: bool) -> &mut Self {
        self.command_as_array = value;
        self
    }
}

/// Represents a JSON compilation database.
pub struct Database {
    path: path::PathBuf,
}

impl Database {
    pub fn new(path: &path::Path) -> Self {
        Database { path: path.to_path_buf(), }
    }

    pub fn load() -> Result<Entries> {
        unimplemented!()
    }

    pub fn save(_entries: &Entries, _format: DatabaseFormat) -> Result<()> {
        unimplemented!()
    }
}


mod inner {
    use super::*;
    use serde_json;

    #[derive(Debug, Serialize, Deserialize)]
    struct StringEntry {
        directory: String,
        file: String,
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ArrayEntry {
        directory: String,
        file: String,
        arguments: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum GenericEntry {
        StringEntry(StringEntry),
        ArrayEntry(ArrayEntry),
    }

    type GenericEntries = Vec<GenericEntry>;


    fn load(_path: &path::Path) -> Result<GenericEntries> {
        unimplemented!()
    }

    fn save(_path: &path::Path, _entries: &GenericEntries) -> Result<()> {
        unimplemented!()
    }

    fn from(_entry: Entry, _format: DatabaseFormat) -> GenericEntry {
        unimplemented!()
    }

    fn into(_entry: GenericEntry) -> Entry {
        unimplemented!()
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_load_arguments() {
            let input =
                r#"{
                "directory": "/build/dir/path",
                "file": "/path/to/source/file.c",
                "arguments": ["cc", "-c", "/path/to/source/file.c"]
            }"#;

            let entry: GenericEntry = serde_json::from_str(input).unwrap();
            println!("{:?}", entry);
        }

        #[test]
        fn test_save_arguments() {
            let entry_one = GenericEntry::ArrayEntry(ArrayEntry {
                directory: "/build/dir/path".to_string(),
                file: "/path/to/source.c".to_string(),
                arguments: vec!["cc".to_string(), "-c".to_string()],
                output: None
            });
            let entry_two = GenericEntry::StringEntry(StringEntry {
                directory: "/build/dir/path".to_string(),
                file: "/path/to/source.c".to_string(),
                command: "cc -c /path/to/source.c -o /build/dir/path/source.o".to_string(),
                output: Some("/build/dir/path/source.o".to_string())
            });
            let inputs = vec![entry_one, entry_two];

            let output = serde_json::to_string(&inputs).unwrap();
            println!("{}", output);
        }
    }
}

//impl Database {
//    pub fn new() -> Database {
//        Database {
//            entries: collections::HashSet::new(),
//        }
//    }
//
//    pub fn load(&mut self, source: &mut io::Read) -> Result<()> {
//        let entries: Vec<Entry> = serde_json::from_reader(source)?;
//        let result = self.add_entries(entries);
//        Ok(result)
//    }
//
//    pub fn save(&self, target: &mut io::Write) -> Result<()> {
//        let values = Vec::from_iter(self.entries.iter());
//        let result = serde_json::to_writer(target, &values)?;
//        Ok(result)
//    }
//
//    pub fn add_entry(&mut self, entry: Entry) -> () {
//        self.entries.insert(entry);
//    }
//
//    pub fn add_entries(&mut self, entries: Vec<Entry>) -> () {
//        let fresh: collections::HashSet<Entry> = collections::HashSet::from_iter(entries);
//        self.entries.union(&fresh);
//    }
//}

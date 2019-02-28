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

use crate::Result;
use crate::compilation::CompilerCall;
use crate::compilation::compiler::CompilerFilter;
use crate::compilation::flags::FlagFilter;
use crate::compilation::source::SourceFilter;
use crate::protocol::collector::Protocol;


/// Represents a compilation database building strategy.
pub struct BuildStrategy {
    pub format: Format,
    pub append_to_existing: bool,
    pub include_headers: bool,
    pub include_linking: bool,
    pub compilers: CompilerFilter,
    pub sources: SourceFilter,
    pub flags: FlagFilter,
}

impl BuildStrategy {

    pub fn build(&self, collector: &Protocol, path: &path::Path) -> Result<()> {
        let current: Entries = collector.events()
            .filter_map(|event| {
                debug!("Intercepted event: {:?}", event);
                event.to_execution()
            })
            .filter_map(|execution| {
                debug!("Intercepted execution: {:?} @ {:?}", execution.0, execution.1);
                CompilerCall::from(&execution.0, execution.1.as_ref()).ok()
            })
            .flat_map(|call| {
                debug!("Intercepted compiler call: {:?}", call);
                Entry::from(&call, &self.format)
            })
            .collect();

        let db = Database::new(path);
        if self.append_to_existing {
            let previous = db.load()?;
            db.save(previous.union(&current), &self.format)
        } else {
            db.save(current.iter(), &self.format)
        }
    }

    pub fn transform(&self, _path: &path::Path) -> Result<()> {
        unimplemented!()
    }
}

impl Default for BuildStrategy {
    fn default() -> Self {
        unimplemented!()
    }
}

/// Represents the expected format of the JSON compilation database.
pub struct Format {
    pub relative_to: Option<path::PathBuf>,
    pub command_as_array: bool,
    pub drop_output_field: bool,
    pub drop_wrapper: bool,
}

impl Default for Format {
    fn default() -> Self {
        Format {
            relative_to: None,
            command_as_array: true,
            drop_output_field: false,
            drop_wrapper: true,
        }
    }
}


/// Represents a generic entry of the compilation database.
#[derive(Hash, Debug)]
pub struct Entry {
    pub directory: path::PathBuf,
    pub file: path::PathBuf,
    pub command: Vec<String>,
    pub output: Option<path::PathBuf>,
}

impl Entry {
    pub fn from(compilation: &CompilerCall, format: &Format) -> Vec<Entry> {
        entry::from(compilation, format)
    }
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

pub type Entries = collections::HashSet<Entry>;


/// Represents a JSON compilation database.
pub struct Database {
    path: path::PathBuf,
}

impl Database {
    pub fn new(path: &path::Path) -> Self {
        Database { path: path.to_path_buf(), }
    }

    pub fn load(&self) -> Result<Entries> {
        db::load(self)
    }

    pub fn save<'a, I>(&self, entries: I, format: &Format) -> Result<()>
        where I: Iterator<Item = &'a Entry>
    {
        db::save(self, entries, format)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};

    macro_rules! vec_of_strings {
        ($($x:expr),*) => (vec![$($x.to_string()),*]);
    }

    #[test]
    #[should_panic]
    fn test_load_not_existing_file_fails() {
        let sut = Database::new(path::Path::new("/not/exists/file.json"));
        let _ = sut.load().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_load_json_failed() {
        let comp_db_file = TestFile::new()
            .expect("test file setup failed");
        comp_db_file.write(br#"this is not json"#)
            .expect("test file content write failed");

        let sut = Database::new(comp_db_file.path());
        let _ = sut.load().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_load_not_expected_json_failed() {
        let comp_db_file = TestFile::new()
            .expect("test file setup failed");
        comp_db_file.write(br#"{ "file": "string" }"#)
            .expect("test file content write failed");

        let sut = Database::new(comp_db_file.path());
        let _ = sut.load().unwrap();
    }

    #[test]
    fn test_load_empty() -> Result<()> {
        let comp_db_file = TestFile::new()?;
        comp_db_file.write(br#"[]"#)?;

        let sut = Database::new(comp_db_file.path());
        let entries = sut.load()?;

        let expected = Entries::new();
        assert_eq!(expected, entries);
        Ok(())
    }

    #[test]
    fn test_load_string_command() -> Result<()> {
        let comp_db_file = TestFile::new()?;
        comp_db_file.write(
            br#"[
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "command": "cc -c ./file_a.c -o ./file_a.o"
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "output": "./file_b.o",
                    "command": "cc -c ./file_b.c -o ./file_b.o"
                }
            ]"#
        )?;

        let sut = Database::new(comp_db_file.path());
        let entries = sut.load()?;

        let expected = expected_values();
        assert_eq!(expected, entries);
        Ok(())
    }

    #[test]
    fn test_load_array_command() -> Result<()> {
        let comp_db_file = TestFile::new()?;
        comp_db_file.write(
            br#"[
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "arguments": ["cc", "-c", "./file_a.c", "-o", "./file_a.o"]
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "output": "./file_b.o",
                    "arguments": ["cc", "-c", "./file_b.c", "-o", "./file_b.o"]
                }
            ]"#
        )?;

        let sut = Database::new(comp_db_file.path());
        let entries = sut.load()?;

        let expected = expected_values();
        assert_eq!(expected, entries);
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_load_path_problem() {
        let comp_db_file = TestFile::new()
            .expect("test file setup failed");
        comp_db_file.write(br#"[
                {
                    "directory": " ",
                    "file": "./file_a.c",
                    "command": "cc -Dvalue=\"this"
                }
            ]"#)
            .expect("test file content write failed");

        let sut = Database::new(comp_db_file.path());
        let _ = sut.load().unwrap();
    }

    #[test]
    fn test_save_string_command() -> Result<()> {
        let comp_db_file = TestFile::new()?;

        let sut = Database::new(comp_db_file.path());
        let formatter = Format { command_as_array: false, ..Format::default() };

        let expected = expected_values();
        sut.save(expected.iter(), &formatter)?;

        let entries = sut.load()?;

        let expected = expected_values();
        assert_eq!(expected, entries);

        let content = comp_db_file.read()?;
        println!("{}", content);

        Ok(())
    }

    #[test]
    fn test_save_array_command() -> Result<()> {
        let comp_db_file = TestFile::new()?;

        let sut = Database::new(comp_db_file.path());
        let formatter = Format { command_as_array: true, ..Format::default() };

        let expected = expected_values();
        sut.save(expected.iter(), &formatter)?;

        let entries = sut.load()?;

        let expected = expected_values();
        assert_eq!(expected, entries);

        let content = comp_db_file.read()?;
        println!("{}", content);

        Ok(())
    }

    #[allow(dead_code)]
    struct TestFile {
        directory: tempfile::TempDir,
        file: path::PathBuf,
    }

    impl TestFile {

        pub fn new() -> Result<TestFile> {
            let directory = tempfile::Builder::new()
                .prefix("bear-test-")
                .rand_bytes(12)
                .tempdir()?;

            let mut file = directory.path().to_path_buf();
            file.push("comp-db.json");

            Ok(TestFile { directory, file })
        }

        pub fn path(&self) -> &path::Path {
            self.file.as_path()
        }

        pub fn write(&self, content: &[u8]) -> Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(self.path())?;

            file.write(content)?;
            Ok(())
        }

        pub fn read(&self) -> Result<String> {
            let mut file = fs::OpenOptions::new()
                .read(true)
                .open(self.path())?;

            let mut result = String::new();
            file.read_to_string(&mut result)?;
            Ok(result)
        }
    }

    fn expected_values() -> Entries {
        let mut expected: Entries = collections::HashSet::new();
        expected.insert(
            Entry {
                directory: path::PathBuf::from("/home/user"),
                file: path::PathBuf::from("./file_a.c"),
                command: vec_of_strings!("cc", "-c", "./file_a.c", "-o", "./file_a.o"),
                output: None,
            }
        );
        expected.insert(
            Entry {
                directory: path::PathBuf::from("/home/user"),
                file: path::PathBuf::from("./file_b.c"),
                command: vec_of_strings!("cc", "-c", "./file_b.c", "-o", "./file_b.o"),
                output: Some(path::PathBuf::from("./file_b.o")),
            }
        );
        expected
    }
}


mod db {
    use super::*;
    use std::fs;
    use serde_json;
    use shellwords;

    pub fn load(db: &Database) -> Result<Entries> {
        let generic_entries = read(&db.path)?;
        let entries = generic_entries.iter()
            .map(|entry| into(entry))
            .collect::<Result<Entries>>();
        // In case of error, let's be verbose which entries were problematic.
        if let Err(_) = entries {
            let errors = generic_entries.iter()
                .map(|entry| into(entry))
                .filter_map(Result::err)
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(errors.into())
        } else {
            entries
        }
    }

    pub fn save<'a, I>(db: &Database, entries: I, format: &Format) -> Result<()>
        where I: Iterator<Item = &'a Entry>
    {
        let generic_entries = entries
            .map(|entry| from(entry, format))
            .collect::<Result<Vec<_>>>()?;
        write(&db.path, &generic_entries)
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(untagged)]
    enum GenericEntry {
        StringEntry {
            directory: String,
            file: String,
            command: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            output: Option<String>,
        },
        ArrayEntry {
            directory: String,
            file: String,
            arguments: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            output: Option<String>,
        },
    }

    type GenericEntries = Vec<GenericEntry>;

    fn read(path: &path::Path) -> Result<GenericEntries> {
        let file = fs::OpenOptions::new()
            .read(true)
            .open(path)?;
        let entries: GenericEntries = serde_json::from_reader(file)?;
        Ok(entries)
    }

    fn write(path: &path::Path, entries: &GenericEntries) -> Result<()> {
        let file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;
        serde_json::ser::to_writer_pretty(file, entries)
            .map_err(|error| error.into())
    }

    fn from(entry: &Entry, format: &Format) -> Result<GenericEntry> {
        fn path_to_string(path: &path::Path) -> Result<String> {
            match path.to_str() {
                Some(str) => Ok(str.to_string()),
                None => Err(format!("Failed to convert to string {:?}", path).into()),
            }
        }

        let directory = path_to_string(entry.directory.as_path())?;
        let file = path_to_string(entry.file.as_path())?;
        let output = match entry.output {
            Some(ref path) => path_to_string(path).map(Option::Some),
            None => Ok(None),
        }?;
        if format.command_as_array {
            Ok(GenericEntry::ArrayEntry {
                directory,
                file,
                arguments: entry.command.clone(),
                output
            })
        } else {
            Ok(GenericEntry::StringEntry {
                directory,
                file,
                command: shellwords::join(
                    entry.command
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .as_ref()),
                output
            })
        }
    }

    fn into(entry: &GenericEntry) -> Result<Entry> {
        match entry {
            GenericEntry::ArrayEntry { directory, file, arguments, output } => {
                let directory_path = path::PathBuf::from(directory);
                let file_path = path::PathBuf::from(file);
                let output_path = output.clone().map(|string| path::PathBuf::from(string));
                Ok(Entry {
                    directory: directory_path,
                    file: file_path,
                    command: arguments.clone(),
                    output: output_path,
                })
            },
            GenericEntry::StringEntry { directory, file, command, output } => {
                match shellwords::split(command) {
                    Ok(arguments) => {
                        let directory_path = path::PathBuf::from(directory);
                        let file_path = path::PathBuf::from(file);
                        let output_path = output.clone().map(|string| path::PathBuf::from(string));
                        Ok(Entry {
                            directory: directory_path,
                            file: file_path,
                            command: arguments,
                            output: output_path,
                        })
                    },
                    Err(_) =>
                        Err(format!("Quotes are mismatch in {:?}", command).into()),
                }
            }
        }
    }
}

mod entry {
    use super::*;

    pub fn from(compilation: &CompilerCall, format: &Format) -> Vec<Entry> {
        let make_output= |source: &path::PathBuf| {
            match compilation.output() {
                None =>
                    source.with_extension(
                        source.extension()
                            .map(|e| {
                                let mut result = e.to_os_string();
                                result.push(".o");
                                result
                            })
                            .unwrap_or(std::ffi::OsString::from("o"))),
                Some(o) =>
                    o.to_path_buf(),
            }
        };

        let make_command = |source: &path::PathBuf, output: &path::PathBuf| {
            let mut result = compilation.compiler().to_strings(format.drop_wrapper);
            result.push(compilation.pass().to_string());
            result.append(&mut compilation.flags());
            result.push(source.to_string_lossy().into_owned());
            result.push("-o".to_string());
            result.push(output.to_string_lossy().into_owned());
            result
        };

        if compilation.pass().is_compiling() {
            compilation.sources()
                .iter()
                .map(|source| {
                    let output = make_output(source);
                    let command = make_command(source, &output);
                    Entry {
                        directory: compilation.work_dir.clone(),
                        file: source.to_path_buf(),
                        output: Some(output),
                        command,
                    }
                })
                .collect::<Vec<Entry>>()
        } else {
            vec!()
        }
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! This crate provides support for reading and writing JSON compilation database files.
//!
//! A compilation database is a set of records which describe the compilation of the
//! source files in a given project. It describes the compiler invocation command to
//! compile a source module to an object file.
//!
//! This database can have many forms. One well known and supported format is the JSON
//! compilation database, which is a simple JSON file having the list of compilation
//! as an array. The definition of the JSON compilation database files is done in the
//! LLVM project [documentation](https://clang.llvm.org/docs/JSONCompilationDatabase.html).

use serde::ser::{SerializeSeq, Serializer};
use serde::Serialize;
use serde_json::Error;

mod iterator;
mod tests;
mod type_de;
mod type_ser;

/// Represents an entry of the compilation database.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Entry {
    /// The main translation unit source processed by this compilation step.
    /// This is used by tools as the key into the compilation database.
    /// There can be multiple command objects for the same file, for example if the same
    /// source file is compiled with different configurations.
    pub file: std::path::PathBuf,
    /// The compile command executed. This must be a valid command to rerun the exact
    /// compilation step for the translation unit in the environment the build system uses.
    /// Shell expansion is not supported.
    pub arguments: Vec<String>,
    /// The working directory of the compilation. All paths specified in the command or
    /// file fields must be either absolute or relative to this directory.
    pub directory: std::path::PathBuf,
    /// The name of the output created by this compilation step. This field is optional.
    /// It can be used to distinguish different processing modes of the same input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<std::path::PathBuf>,
}

/// Deserialize entries from a JSON array into an iterator.
pub fn read(reader: impl std::io::Read) -> impl Iterator<Item = Result<Entry, Error>> {
    iterator::iter_json_array(reader)
}

/// The entries are written in the format specified by the configuration.
pub fn write(
    command_as_array: bool,
    writer: impl std::io::Write,
    entries: impl Iterator<Item = Entry>,
) -> Result<(), Error> {
    let method = if command_as_array {
        write_with_arguments
    } else {
        write_with_command
    };
    method(writer, entries)
}

/// Serialize entries from an iterator into a JSON array.
///
/// It uses the `arguments` field of the `Entry` struct to serialize the array of strings.
pub(super) fn write_with_arguments(
    writer: impl std::io::Write,
    entries: impl Iterator<Item = Entry>,
) -> Result<(), Error> {
    let mut ser = serde_json::Serializer::pretty(writer);
    let mut seq = ser.serialize_seq(None)?;
    for entry in entries {
        seq.serialize_element(&entry)?;
    }
    seq.end()
}

/// Serialize entries from an iterator into a JSON array.
///
/// It uses the `arguments` field of the `Entry` struct to serialize the array of strings.
pub(super) fn write_with_command(
    writer: impl std::io::Write,
    entries: impl Iterator<Item = Entry>,
) -> Result<(), Error> {
    let mut ser = serde_json::Serializer::pretty(writer);
    let mut seq = ser.serialize_seq(None)?;
    for entry in entries {
        let entry = type_ser::EntryWithCommand::from(entry);
        seq.serialize_element(&entry)?;
    }
    seq.end()
}

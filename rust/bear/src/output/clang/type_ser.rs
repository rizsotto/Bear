// SPDX-License-Identifier: GPL-3.0-or-later

//! Implements serialization of the `Entry` struct.

use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::Entry;

impl Serialize for Entry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let size = if self.output.is_some() { 4 } else { 3 };
        let mut state = serializer.serialize_struct("Entry", size)?;
        state.serialize_field("directory", &self.directory)?;
        state.serialize_field("file", &self.file)?;
        state.serialize_field("arguments", &self.arguments)?;
        if self.output.is_some() {
            state.serialize_field("output", &self.output)?;
        }
        state.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryWithCommand {
    pub file: std::path::PathBuf,
    pub command: String,
    pub directory: std::path::PathBuf,
    pub output: Option<std::path::PathBuf>,
}

impl From<Entry> for EntryWithCommand {
    fn from(entry: Entry) -> Self {
        Self {
            file: entry.file,
            command: shell_words::join(&entry.arguments),
            directory: entry.directory,
            output: entry.output,
        }
    }
}

impl Serialize for EntryWithCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let size = if self.output.is_some() { 4 } else { 3 };
        let mut state = serializer.serialize_struct("Entry", size)?;
        state.serialize_field("directory", &self.directory)?;
        state.serialize_field("file", &self.file)?;
        state.serialize_field("command", &self.command)?;
        if self.output.is_some() {
            state.serialize_field("output", &self.output)?;
        }
        state.end()
    }
}

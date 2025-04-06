// SPDX-License-Identifier: GPL-3.0-or-later

//! Implements serialization of the `Entry` struct.

use super::Entry;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EntryWithCommand {
    pub file: std::path::PathBuf,
    pub command: String,
    pub directory: std::path::PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
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

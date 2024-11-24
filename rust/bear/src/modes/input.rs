// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Context;
use serde_json::de::IoRead;
use serde_json::{Error, StreamDeserializer};
use std::fs::OpenOptions;
use std::io::BufReader;
use std::path::PathBuf;

use crate::args;
use crate::ipc::{Envelope, Execution};

/// Responsible for reading the build events from the intercept mode.
///
/// The file syntax is defined by the `events` module, and the parsing logic is implemented there.
/// Here we only handle the file opening and the error handling.
pub struct EventFileReader {
    stream: Box<dyn Iterator<Item = Result<Envelope, Error>>>,
}

impl TryFrom<args::BuildEvents> for EventFileReader {
    type Error = anyhow::Error;

    /// Open the file and create a new instance of the event file reader.
    ///
    /// If the file cannot be opened, the error will be logged and escalated.
    fn try_from(value: args::BuildEvents) -> Result<Self, Self::Error> {
        let file_name = PathBuf::from(value.file_name);
        let file = OpenOptions::new()
            .read(true)
            .open(file_name.as_path())
            .map(BufReader::new)
            .with_context(|| format!("Failed to open input file: {:?}", file_name))?;
        let stream = Box::new(StreamDeserializer::new(IoRead::new(file)));

        Ok(EventFileReader { stream })
    }
}

impl EventFileReader {
    /// Generate the build events from the file.
    ///
    /// Returns an iterator over the build events. Any error during the reading
    /// of the file will be logged and the failed entries will be skipped.
    pub fn generate(self) -> impl Iterator<Item = Execution> {
        self.stream.filter_map(|result| match result {
            Ok(value) => Some(value.event.execution),
            Err(error) => {
                log::error!("Failed to read event: {:?}", error);
                None
            }
        })
    }
}

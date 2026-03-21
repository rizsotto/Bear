// SPDX-License-Identifier: GPL-3.0-or-later

//! Serialization format for execution events.
//!
//! The format is a [JSON line format](https://jsonlines.org/), which is a sequence
//! of JSON objects separated by newlines.
//!
//! # Note
//! The output format is not stable and may change in future versions.

use super::{SerializationError, SerializationFormat};
use crate::intercept;
use serde_json::StreamDeserializer;
use serde_json::de::IoRead;

/// The type represents a database format for execution events.
pub struct ExecutionEventDatabase;

impl SerializationFormat<intercept::Event> for ExecutionEventDatabase {
    fn write(
        writer: impl std::io::Write,
        events: impl Iterator<Item = intercept::Event>,
    ) -> Result<(), SerializationError> {
        let mut writer = writer;
        for event in events {
            serde_json::to_writer(&mut writer, &event).map_err(SerializationError::Syntax)?;
            writer.write_all(b"\n").map_err(SerializationError::Io)?;
        }
        Ok(())
    }

    fn read(
        reader: impl std::io::Read,
    ) -> impl Iterator<Item = Result<intercept::Event, SerializationError>> {
        let stream = StreamDeserializer::new(IoRead::new(reader));
        stream.map(|value| value.map_err(SerializationError::Syntax))
    }
}

#[cfg(test)]
mod tests {
    use super::ExecutionEventDatabase as Sut;
    use super::SerializationFormat;
    use crate::intercept::Event;
    use serde_json::json;
    use std::collections::HashMap;
    use std::io::{Cursor, Seek, SeekFrom};

    #[test]
    fn read_write() {
        let events = expected_values();

        let mut buffer = Cursor::new(Vec::new());
        Sut::write(&mut buffer, events.iter().cloned()).unwrap();

        buffer.seek(SeekFrom::Start(0)).unwrap();
        let read_events: Vec<_> = Sut::read(&mut buffer).collect::<Result<_, _>>().unwrap();

        assert_eq!(events, read_events);
    }

    #[test]
    fn read_write_empty() {
        let events = Vec::<Event>::new();

        let mut buffer = Cursor::new(Vec::new());
        Sut::write(&mut buffer, events.iter().cloned()).unwrap();

        buffer.seek(SeekFrom::Start(0)).unwrap();
        let read_events: Vec<_> = Sut::read(&mut buffer).collect::<Result<_, _>>().unwrap();

        assert_eq!(events, read_events);
    }

    #[test]
    fn read_stops_on_errors() {
        let line1 = json!({
            "pid": 11782,
            "execution": {
                "executable": "/usr/bin/clang",
                "arguments": ["clang", "-c", "main.c"],
                "working_dir": "/home/user",
                "environment": {
                    "PATH": "/usr/bin",
                    "HOME": "/home/user"
                }
            }
        });
        let line2 = json!({"rid": 42 });
        let line3 = json!({
            "pid": 11934,
            "execution": {
                "executable": "/usr/bin/clang",
                "arguments": ["clang", "-c", "output.c"],
                "working_dir": "/home/user",
                "environment": {}
            }
        });
        let content = format!("{line1}\n{line2}\n{line3}\n");

        let mut cursor = Cursor::new(content);
        let warnings = std::cell::RefCell::new(Vec::new());
        let read_events: Vec<_> = Sut::read_and_ignore(&mut cursor, |error| {
            warnings.borrow_mut().push(format!("Warning: {error:?}"));
        })
        .collect();

        // Only the first event is read, all other lines are ignored.
        assert_eq!(expected_values()[0..1], read_events);
        assert_eq!(warnings.borrow().len(), 1);
    }

    fn expected_values() -> Vec<Event> {
        vec![
            Event::from_strings(
                11782,
                "/usr/bin/clang",
                vec!["clang", "-c", "main.c"],
                "/home/user",
                HashMap::from([("PATH", "/usr/bin"), ("HOME", "/home/user")]),
            ),
            Event::from_strings(
                11934,
                "/usr/bin/clang",
                vec!["clang", "-c", "output.c"],
                "/home/user",
                HashMap::from([]),
            ),
        ]
    }
}

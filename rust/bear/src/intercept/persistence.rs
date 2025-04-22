// SPDX-License-Identifier: GPL-3.0-or-later

use super::Event;
use serde_json::de::IoRead;
use serde_json::StreamDeserializer;
use std::io;

/// Generate the build events from the file.
///
/// Returns an iterator over the build events.
/// Any error will interrupt the reading process and the remaining events will be lost.
pub fn read(reader: impl io::Read) -> impl Iterator<Item = Event> {
    let stream = StreamDeserializer::new(IoRead::new(reader));
    stream.filter_map(|result| match result {
        Ok(value) => Some(value),
        Err(error) => {
            log::error!("Failed to read event: {:?}", error);
            None
        }
    })
}

/// Write the build events to the file.
///
/// Can fail if the events cannot be serialized or written to the file.
/// Any error will interrupt the writing process and the file will be incomplete.
pub fn write(
    mut writer: impl io::Write,
    events: impl IntoIterator<Item = Event>,
) -> Result<(), anyhow::Error> {
    for event in events {
        serde_json::to_writer(&mut writer, &event)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::intercept::{Event, Execution, ProcessId};
    use crate::vec_of_strings;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn read_write() {
        let events = expected_values();

        let mut buffer = Vec::new();
        write(&mut buffer, events.iter().cloned()).unwrap();
        let mut cursor = io::Cursor::new(buffer);
        let read_events: Vec<_> = read(&mut cursor).collect();

        assert_eq!(events, read_events);
    }

    #[test]
    fn read_write_empty() {
        let events = Vec::<Event>::new();

        let mut buffer = Vec::new();
        write(&mut buffer, events.iter().cloned()).unwrap();
        let mut cursor = io::Cursor::new(buffer);
        let read_events: Vec<_> = read(&mut cursor).collect();

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
        let content = format!("{}\n{}\n{}\n", line1, line2, line3);

        let mut cursor = io::Cursor::new(content);
        let read_events: Vec<_> = read(&mut cursor).collect();

        // Only the fist event is read, all other lines are ignored.
        assert_eq!(expected_values()[0..1], read_events);
    }

    fn expected_values() -> Vec<Event> {
        vec![
            Event {
                pid: ProcessId(11782),
                execution: Execution {
                    executable: PathBuf::from("/usr/bin/clang"),
                    arguments: vec_of_strings!["clang", "-c", "main.c"],
                    working_dir: PathBuf::from("/home/user"),
                    environment: HashMap::from([
                        ("PATH".to_string(), "/usr/bin".to_string()),
                        ("HOME".to_string(), "/home/user".to_string()),
                    ]),
                },
            },
            Event {
                pid: ProcessId(11934),
                execution: Execution {
                    executable: PathBuf::from("/usr/bin/clang"),
                    arguments: vec_of_strings!["clang", "-c", "output.c"],
                    working_dir: PathBuf::from("/home/user"),
                    environment: HashMap::from([]),
                },
            },
        ]
    }
}

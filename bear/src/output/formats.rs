// SPDX-License-Identifier: GPL-3.0-or-later

//! This module declares the file formats used by this project.
//! The following formats are declared:
//!
//! - The JSON compilation database format, as declared by the Clang project.
//! - The semantic database format (internal format of this project).
//! - The execution event database format (internal format of this project.)

use super::{clang, json};
use crate::intercept;
use serde_json::StreamDeserializer;
use serde_json::de::IoRead;
use thiserror::Error;

/// Represents errors that can occur while working with file formats.
#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("Generic IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Format syntax error: {0}")]
    Syntax(#[from] serde_json::Error),
    #[error("Format semantic error: {0}")]
    Semantic(#[from] clang::EntryError),
}

/// A trait representing a file format that can be written to and read from.
///
/// File formats in this project are usually sequences of values. This trait
/// provides a type-independent abstraction over file formats.
pub trait SerializationFormat<T> {
    /// Writes an iterator of items to the specified writer.
    fn write(writer: impl std::io::Write, items: impl Iterator<Item = T>) -> Result<(), SerializationError>;

    /// Reads items from the specified reader, returning an iterator of results.
    fn read(reader: impl std::io::Read) -> impl Iterator<Item = Result<T, SerializationError>>;

    /// Reads entries from the file and ignores any errors.
    ///
    /// This is not always feasible when the file format is strict.
    fn read_and_ignore(reader: impl std::io::Read, message_writer: impl Fn(&str)) -> impl Iterator<Item = T> {
        Self::read(reader).filter_map(move |result| match result {
            Ok(value) => Some(value),
            Err(error) => {
                message_writer(&error.to_string());
                None
            }
        })
    }
}

/// The type represents a JSON compilation database format.
///
/// The format is a JSON array format, which is a sequence of JSON objects
/// enclosed in square brackets. Each object represents a compilation
/// command.
///
/// # Note
/// The format itself is defined in the LLVM project documentation.
/// https://clang.llvm.org/docs/JSONCompilationDatabase.html
pub struct JsonCompilationDatabase;

impl SerializationFormat<clang::Entry> for JsonCompilationDatabase {
    fn write(
        writer: impl std::io::Write,
        entries: impl Iterator<Item = clang::Entry>,
    ) -> Result<(), SerializationError> {
        json::serialize_result_seq(
            writer,
            // Ensure only valid entries are serialized.
            entries.map(|entry| match entry.validate() {
                Ok(_) => Ok(entry),
                Err(err) => Err(SerializationError::Semantic(err)),
            }),
        )
    }

    fn read(reader: impl std::io::Read) -> impl Iterator<Item = Result<clang::Entry, SerializationError>> {
        json::deserialize_seq(reader).map(|res| {
            res.map_err(SerializationError::Syntax)
                // Ensure only valid entries are returned.
                .and_then(|entry: clang::Entry| match entry.validate() {
                    Ok(_) => Ok(entry),
                    Err(err) => Err(SerializationError::Semantic(err)),
                })
        })
    }
}

/// The type represents a database format for execution events.
///
/// The format is a [JSON line format](https://jsonlines.org/), which is a sequence
/// of JSON objects separated by newlines.
///
/// # Note
/// The output format is not stable and may change in future versions.
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
mod test {
    mod compilation_database {
        use super::super::JsonCompilationDatabase as Sut;
        use super::super::clang::{Entry, EntryError};
        use super::super::{SerializationError, SerializationFormat};
        use serde_json::error::Category;
        use serde_json::json;
        use std::io::{Cursor, Seek, SeekFrom};

        macro_rules! assert_json_error {
            ($x:expr) => {
                match $x {
                    Some(Err(SerializationError::Syntax(error))) => {
                        assert_eq!(error.classify(), Category::Data)
                    }
                    _ => assert!(false, "shout be JSON error"),
                }
            };
        }

        macro_rules! assert_format_error {
            ($x:expr) => {
                assert!(matches!($x, Some(Err(SerializationError::Semantic(_)))), "should be format error");
            };
        }

        #[test]
        fn load_non_json_content() {
            let content = r#"this is not json"#;
            let mut result = Sut::read(content.as_bytes());

            assert_json_error!(result.next());
            assert!(result.next().is_none());
        }

        #[test]
        fn load_not_expected_json_content() {
            let content = json!({ "file": "string" }).to_string();
            let mut result = Sut::read(content.as_bytes());

            assert_json_error!(result.next());
            assert!(result.next().is_none());
        }

        #[test]
        fn load_on_bad_value() {
            let content = json!([
                {
                    "directory": " ",
                    "file": "./file_a.c",
                    "command": "cc -Dvalue=\"this"
                }
            ])
            .to_string();
            let mut result = Sut::read(content.as_bytes());

            assert_format_error!(result.next());
            assert!(result.next().is_none());
        }

        #[test]
        fn load_on_multiple_commands() {
            let content = json!([
                {
                    "directory": " ",
                    "file": "./file_a.c",
                    "command": "cc source.c",
                    "arguments": ["cc", "source.c"],
                }
            ])
            .to_string();
            let mut result = Sut::read(content.as_bytes());

            assert_format_error!(result.next());
            assert!(result.next().is_none());
        }

        #[test]
        fn load_empty_array() {
            let content = json!([]).to_string();

            let mut result = Sut::read(content.as_bytes());

            assert!(result.next().is_none());
        }

        #[test]
        fn write_fails_on_invalid_entry() {
            let entry = Entry::from_arguments_str("main.cpp", vec!["clang", "-c"], "", None);
            assert_eq!(entry.validate(), Err(EntryError::EmptyDirectory));

            let mut buffer = Cursor::new(Vec::new());
            let result = Sut::write(&mut buffer, vec![entry].into_iter());

            assert!(result.is_err());
            assert!(matches!(result, Err(SerializationError::Semantic(_))));
        }

        fn expected_values_with_arguments() -> Vec<Entry> {
            vec![
                Entry::from_arguments_str(
                    "./file_a.c",
                    vec!["cc", "-c", "./file_a.c", "-o", "./file_a.o"],
                    "/home/user",
                    None,
                ),
                Entry::from_arguments_str(
                    "./file_b.c",
                    vec!["cc", "-c", "./file_b.c", "-o", "./file_b.o"],
                    "/home/user",
                    Some("./file_b.o"),
                ),
            ]
        }

        fn expected_values_with_command() -> Vec<Entry> {
            vec![
                Entry::from_command_str("./file_a.c", "cc -c ./file_a.c -o ./file_a.o", "/home/user", None),
                Entry::from_command_str(
                    "./file_b.c",
                    "cc -c ./file_b.c -o ./file_b.o",
                    "/home/user",
                    Some("./file_b.o"),
                ),
            ]
        }

        fn expected_with_array_syntax() -> serde_json::Value {
            json!([
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
            ])
        }

        fn expected_with_string_syntax() -> serde_json::Value {
            json!([
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
            ])
        }

        #[test]
        fn load_content_with_string_command_syntax() {
            let content = expected_with_string_syntax().to_string();

            let result = Sut::read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_values_with_command(), entries);
        }

        #[test]
        fn load_content_with_array_command_syntax() {
            let content = expected_with_array_syntax().to_string();

            let result = Sut::read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_values_with_arguments(), entries);
        }

        #[test]
        fn save_with_array_command_syntax() -> Result<(), SerializationError> {
            let input = expected_values_with_arguments();

            // Create fake "file"
            let mut buffer = Cursor::new(Vec::new());
            let result = Sut::write(&mut buffer, input.into_iter());
            assert!(result.is_ok());

            // Use the fake "file" as input
            buffer.seek(SeekFrom::Start(0))?;
            let content: serde_json::Value = serde_json::from_reader(&mut buffer)?;

            assert_eq!(expected_with_array_syntax(), content);

            Ok(())
        }

        fn expected_quoted_values_with_argument() -> Vec<Entry> {
            vec![
                Entry::from_arguments_str(
                    "./file_a.c",
                    vec!["cc", "-c", "-D", r#"name=\"me\""#, "./file_a.c", "-o", "./file_a.o"],
                    "/home/user",
                    None,
                ),
                Entry::from_arguments_str(
                    "./file_b.c",
                    vec!["cc", "-c", "-D", r#"name="me""#, "./file_b.c", "-o", "./file_b.o"],
                    "/home/user",
                    None,
                ),
            ]
        }

        fn expected_quoted_values_with_command() -> Vec<Entry> {
            vec![
                Entry::from_command_str(
                    "./file_a.c",
                    r#"cc -c -D 'name=\"me\"' ./file_a.c -o ./file_a.o"#,
                    "/home/user",
                    None,
                ),
                Entry::from_command_str(
                    "./file_b.c",
                    r#"cc -c -D 'name="me"' ./file_b.c -o ./file_b.o"#,
                    "/home/user",
                    None,
                ),
            ]
        }

        fn expected_quoted_with_array_syntax() -> serde_json::Value {
            json!([
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "arguments": ["cc", "-c", "-D", r#"name=\"me\""#, "./file_a.c", "-o", "./file_a.o"]
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "arguments": ["cc", "-c", "-D", r#"name="me""#, "./file_b.c", "-o", "./file_b.o"]
                }
            ])
        }

        fn expected_quoted_with_string_syntax() -> serde_json::Value {
            json!([
                {
                    "directory": "/home/user",
                    "file": "./file_a.c",
                    "command": r#"cc -c -D 'name=\"me\"' ./file_a.c -o ./file_a.o"#
                },
                {
                    "directory": "/home/user",
                    "file": "./file_b.c",
                    "command": r#"cc -c -D 'name="me"' ./file_b.c -o ./file_b.o"#
                }
            ])
        }

        #[test]
        fn load_quoted_content_with_array_command_syntax() {
            let content = expected_quoted_with_array_syntax().to_string();

            let result = Sut::read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_quoted_values_with_argument(), entries);
        }

        #[test]
        fn load_quoted_content_with_string_command_syntax() {
            let content = expected_quoted_with_string_syntax().to_string();

            let result = Sut::read(content.as_bytes());
            let entries: Vec<Entry> = result.map(|e| e.unwrap()).collect();

            assert_eq!(expected_quoted_values_with_command(), entries);
        }

        #[test]
        fn save_quoted_with_array_command_syntax() -> Result<(), SerializationError> {
            let input = expected_quoted_values_with_argument();

            // Create fake "file"
            let mut buffer = Cursor::new(Vec::new());
            let result = Sut::write(&mut buffer, input.into_iter());
            assert!(result.is_ok());

            // Use the fake "file" as input
            buffer.seek(SeekFrom::Start(0))?;
            let content: serde_json::Value = serde_json::from_reader(&mut buffer)?;

            assert_eq!(expected_quoted_with_array_syntax(), content);

            Ok(())
        }
    }

    mod execution_events {
        use super::super::ExecutionEventDatabase as Sut;
        use super::super::SerializationFormat;
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
}

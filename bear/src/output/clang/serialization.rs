// SPDX-License-Identifier: GPL-3.0-or-later

//! JSON compilation database serialization format.
//!
//! The format is a JSON array format, which is a sequence of JSON objects
//! enclosed in square brackets. Each object represents a compilation command.
//!
//! The format itself is defined in the LLVM project documentation.
//! https://clang.llvm.org/docs/JSONCompilationDatabase.html

use super::Entry;
use super::json;
use crate::output::{SerializationError, SerializationFormat};

/// The type represents a JSON compilation database format.
pub struct JsonCompilationDatabase;

impl SerializationFormat<Entry> for JsonCompilationDatabase {
    /// Serialize entries as a JSON compilation database.
    ///
    /// Entries are expected to already be validated (see
    /// `ValidatingOutputWriter` in the output pipeline). This function
    /// performs no further semantic validation; it reports only I/O and
    /// JSON-encoding errors.
    fn write(
        writer: impl std::io::Write,
        entries: impl Iterator<Item = Entry>,
    ) -> Result<(), SerializationError> {
        json::serialize_seq(writer, entries).map_err(SerializationError::Syntax)
    }

    fn read(reader: impl std::io::Read) -> impl Iterator<Item = Result<Entry, SerializationError>> {
        json::deserialize_seq(reader).map(|res| {
            res.map_err(SerializationError::Syntax)
                // Ensure only valid entries are returned.
                .and_then(|entry: Entry| match entry.validate() {
                    Ok(_) => Ok(entry),
                    Err(err) => Err(SerializationError::Semantic(err)),
                })
        })
    }
}

#[cfg(test)]
mod test {
    use super::JsonCompilationDatabase as Sut;
    use super::{Entry, SerializationError, SerializationFormat};
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

    // Serializer trusts its input; the upstream `ValidatingOutputWriter`
    // filters invalid entries before they reach `write`. See
    // `bear/src/output/writers/validating.rs` for the validation tests.

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

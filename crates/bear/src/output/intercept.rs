// SPDX-License-Identifier: GPL-3.0-or-later

//! Serialization format for execution events.
//!
//! The format is a [JSON line format](https://jsonlines.org/), which is a sequence
//! of JSON objects separated by newlines.
//!
//! # Note
//! The output format is not stable and may change in future versions.

use super::{SerializationError, SerializationFormat};
use std::io::BufRead;

/// The type represents a database format for execution events.
pub struct ExecutionEventDatabase;

impl SerializationFormat<intercept::Execution> for ExecutionEventDatabase {
    fn write(
        writer: impl std::io::Write,
        executions: impl Iterator<Item = intercept::Execution>,
    ) -> Result<(), SerializationError> {
        let mut writer = writer;
        for execution in executions {
            serde_json::to_writer(&mut writer, &execution).map_err(SerializationError::Syntax)?;
            writer.write_all(b"\n").map_err(SerializationError::Io)?;
        }
        Ok(())
    }

    /// Reads line-delimited JSON events, one `Result` per non-blank line.
    ///
    /// Unlike a single `StreamDeserializer` over the whole reader, this
    /// parses each line independently: a malformed line yields `Err` for
    /// that line only, and parsing continues with subsequent lines. This
    /// is required by `docs/requirements/interception-events-format.md`
    /// (a non-conforming line must not silently drop every line after it).
    /// Blank/whitespace-only lines are skipped rather than yielded as
    /// errors or empty results.
    fn read(
        reader: impl std::io::Read,
    ) -> impl Iterator<Item = Result<intercept::Execution, SerializationError>> {
        let buffered = std::io::BufReader::new(reader);
        buffered.lines().filter_map(|line| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => {
                Some(serde_json::from_str::<intercept::Execution>(&line).map_err(SerializationError::Syntax))
            }
            Err(error) => Some(Err(SerializationError::Io(error))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ExecutionEventDatabase as Sut;
    use super::SerializationFormat;
    use intercept::Execution;
    use serde_json::json;
    use std::collections::HashMap;
    use std::io::{Cursor, Seek, SeekFrom};

    #[test]
    fn read_write() {
        let executions = expected_values();

        let mut buffer = Cursor::new(Vec::new());
        Sut::write(&mut buffer, executions.iter().cloned()).unwrap();

        buffer.seek(SeekFrom::Start(0)).unwrap();
        let read_back: Vec<_> = Sut::read(&mut buffer).collect::<Result<_, _>>().unwrap();

        assert_eq!(executions, read_back);
    }

    #[test]
    fn read_write_empty() {
        let executions = Vec::<Execution>::new();

        let mut buffer = Cursor::new(Vec::new());
        Sut::write(&mut buffer, executions.iter().cloned()).unwrap();

        buffer.seek(SeekFrom::Start(0)).unwrap();
        let read_back: Vec<_> = Sut::read(&mut buffer).collect::<Result<_, _>>().unwrap();

        assert_eq!(executions, read_back);
    }

    #[test]
    fn read_continues_past_errors() {
        let line1 = json!({
            "executable": "/usr/bin/clang",
            "arguments": ["clang", "-c", "main.c"],
            "working_dir": "/home/user",
            "environment": {
                "PATH": "/usr/bin",
                "HOME": "/home/user"
            }
        });
        let line2 = json!({"executable": 42 });
        let line3 = json!({
            "executable": "/usr/bin/clang",
            "arguments": ["clang", "-c", "output.c"],
            "working_dir": "/home/user",
            "environment": {}
        });
        let content = format!("{line1}\n{line2}\n{line3}\n");

        let mut cursor = Cursor::new(content);
        let warnings = std::cell::RefCell::new(Vec::new());
        let read_back: Vec<_> = Sut::read_and_ignore(&mut cursor, |error| {
            warnings.borrow_mut().push(format!("Warning: {error:?}"));
        })
        .collect();

        // Both valid executions are read; only the malformed middle line is dropped.
        assert_eq!(expected_values()[0..2], read_back);
        assert_eq!(warnings.borrow().len(), 1);
    }

    #[test]
    fn read_all_malformed_yields_no_executions_and_one_warning_per_line() {
        let content = "{\"executable\": 42}\nnot json at all\n{\"executable\": true}\n".to_string();

        let mut cursor = Cursor::new(content);
        let warnings = std::cell::RefCell::new(Vec::new());
        let read_back: Vec<_> = Sut::read_and_ignore(&mut cursor, |error| {
            warnings.borrow_mut().push(format!("Warning: {error:?}"));
        })
        .collect();

        assert_eq!(Vec::<Execution>::new(), read_back);
        assert_eq!(warnings.borrow().len(), 3);
    }

    fn expected_values() -> Vec<Execution> {
        vec![
            Execution::from_strings(
                "/usr/bin/clang",
                vec!["clang", "-c", "main.c"],
                "/home/user",
                HashMap::from([("PATH", "/usr/bin"), ("HOME", "/home/user")]),
            ),
            Execution::from_strings(
                "/usr/bin/clang",
                vec!["clang", "-c", "output.c"],
                "/home/user",
                HashMap::from([]),
            ),
        ]
    }
}

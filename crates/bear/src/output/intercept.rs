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
        writer.flush().map_err(SerializationError::Io)?;
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
    ///
    /// This is the single production parser for the event stream: it is
    /// what `RawEventReader::produce` (`crates/bear/src/modes/mod.rs`)
    /// consumes directly. `enumerate()` runs before the blank-line
    /// `filter_map`, so a malformed line's `SerializationError::AtLine`
    /// carries its true 1-based physical line number even when earlier
    /// blank lines were skipped.
    fn read(
        reader: impl std::io::Read,
    ) -> impl Iterator<Item = Result<intercept::Execution, SerializationError>> {
        let buffered = std::io::BufReader::new(reader);
        buffered.lines().enumerate().filter_map(|(index, line)| {
            let line_number = index + 1;
            match line {
                Ok(line) if line.trim().is_empty() => None,
                Ok(line) => Some(
                    serde_json::from_str::<intercept::Execution>(&line)
                        .map_err(|source| SerializationError::AtLine { line: line_number, source }),
                ),
                Err(error) => Some(Err(SerializationError::Io(error))),
            }
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

    #[test]
    fn read_reports_physical_line_number_of_malformed_line() {
        let line1 = json!({
            "executable": "/usr/bin/clang",
            "arguments": ["clang", "-c", "main.c"],
            "working_dir": "/home/user",
            "environment": {}
        });
        let line3 = json!({
            "executable": "/usr/bin/clang",
            "arguments": ["clang", "-c", "output.c"],
            "working_dir": "/home/user",
            "environment": {}
        });
        let content = format!("{line1}\nnot json at all\n{line3}\n");

        let mut cursor = Cursor::new(content);
        let results: Vec<_> = Sut::read(&mut cursor).collect();

        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        let error = results[1].as_ref().unwrap_err();
        assert!(error.to_string().contains("line 2"), "expected 'line 2' in: {error}");
        assert!(results[2].is_ok());
    }

    #[test]
    fn read_reports_true_physical_line_number_past_a_blank_line() {
        let line1 = json!({
            "executable": "/usr/bin/clang",
            "arguments": ["clang", "-c", "main.c"],
            "working_dir": "/home/user",
            "environment": {}
        });
        // A blank line between the valid and malformed lines must not shift
        // the reported line number: the malformed line is physical line 3
        // (valid, blank, malformed), not record 2 counting only data lines.
        let content = format!("{line1}\n\nnot json at all\n");

        let mut cursor = Cursor::new(content);
        let results: Vec<_> = Sut::read(&mut cursor).collect();

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        let error = results[1].as_ref().unwrap_err();
        assert!(error.to_string().contains("line 3"), "expected 'line 3' in: {error}");
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

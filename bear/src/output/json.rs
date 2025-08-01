// SPDX-License-Identifier: GPL-3.0-or-later

//! This module contains functions to serialize and deserialize JSON arrays.
//!
//! The main objective is to provide a way to serialize and deserialize
//! entries from an iterator into a JSON array and vice versa. Iterators
//! allow us to process large datasets without loading everything into
//! memory at once.
//!
//! The format these methods produce is a JSON array of objects.
//! It's *not* JSON lines format, which is a sequence of JSON objects
//! separated by newlines.

use std::io;

use serde::de::DeserializeOwned;
use serde::ser::{Serialize, SerializeSeq};
use serde::Serializer;

/// Serialize entries from an iterator into a JSON array.
///
/// The iterator must yield `Result<T, E>` where `T` is the type to be serialized
/// and `E` is the error type. If an error occurs during serialization,
/// the function will return that error.
pub fn serialize_result_seq<W, T, E>(
    writer: W,
    entries: impl Iterator<Item = Result<T, E>>,
) -> Result<(), E>
where
    W: io::Write,
    T: Serialize,
    E: std::error::Error + From<serde_json::Error>,
{
    let mut ser = serde_json::Serializer::pretty(writer);
    let mut seq = ser.serialize_seq(None)?;
    for entry in entries {
        match entry {
            Ok(object) => seq.serialize_element(&object)?,
            Err(err) => return Err(err),
        }
    }
    seq.end()?;

    Ok(())
}

/// Serialize entries from an iterator into a JSON array.
pub fn serialize_seq<W, T>(
    writer: W,
    entries: impl Iterator<Item = T>,
) -> Result<(), serde_json::Error>
where
    W: io::Write,
    T: Serialize,
{
    let mut ser = serde_json::Serializer::pretty(writer);
    let mut seq = ser.serialize_seq(None)?;
    for entry in entries {
        seq.serialize_element(&entry)?;
    }
    seq.end()
}

/// Deserialize entries from a JSON array into an iterator.
///
/// from https://github.com/serde-rs/json/issues/404#issuecomment-892957228
///
/// # Note
/// Works well with self-delimiter-ed types, like string, objects, arrays.
/// Does not work with types like simple integers, floats, etc. The problem
/// is that the deserializer will not be able to distinguish between
/// the end of the type, and it consumes the next byte which is relevant to
/// the array parser.
pub fn deserialize_seq<T, R>(reader: R) -> impl Iterator<Item = Result<T, serde_json::Error>>
where
    T: DeserializeOwned,
    R: io::Read,
{
    let mut reader = PeekableReader::new(reader);
    let mut state = State::AtStart;
    std::iter::from_fn(move || yield_next_obj(&mut reader, &mut state).transpose())
}

// A wrapper around a reader that allows peeking at the next byte without consuming it
struct PeekableReader<R> {
    reader: R,
    peeked: Option<u8>,
}

impl<R: io::Read> PeekableReader<R> {
    fn new(reader: R) -> Self {
        PeekableReader {
            reader,
            peeked: None,
        }
    }

    fn peek(&mut self) -> Result<u8, serde_json::Error> {
        if self.peeked.is_none() {
            let mut byte = 0u8;
            self.reader
                .read_exact(std::slice::from_mut(&mut byte))
                .map_err(serde_json::Error::io)?;
            self.peeked = Some(byte);
        }
        Ok(self.peeked.unwrap())
    }

    fn consume(&mut self) -> Result<u8, serde_json::Error> {
        if let Some(byte) = self.peeked.take() {
            Ok(byte)
        } else {
            let mut byte = 0u8;
            self.reader
                .read_exact(std::slice::from_mut(&mut byte))
                .map_err(serde_json::Error::io)?;
            Ok(byte)
        }
    }

    fn peek_skipping_ws(&mut self) -> Result<u8, serde_json::Error> {
        loop {
            let byte = self.peek()?;
            if !byte.is_ascii_whitespace() {
                return Ok(byte);
            }
            self.consume()?; // Consume whitespace
        }
    }

    fn consume_skipping_ws(&mut self) -> Result<u8, serde_json::Error> {
        self.peek_skipping_ws()?; // Make sure peeked byte is not whitespace
        self.consume()
    }
}

// Implement Read directly for PeekableReader
impl<R: io::Read> io::Read for PeekableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        // If we have a peeked byte, use it first
        if let Some(byte) = self.peeked.take() {
            buf[0] = byte;
            return Ok(1);
        }

        // Otherwise, read directly from the inner reader
        self.reader.read(buf)
    }
}

enum State {
    AtStart,
    AtMiddle,
    Finished,
    Failed,
}

fn yield_next_obj<T, R>(
    reader: &mut PeekableReader<R>,
    state: &mut State,
) -> Result<Option<T>, serde_json::Error>
where
    T: DeserializeOwned,
    R: io::Read,
{
    match state {
        State::AtStart => {
            let bracket = reader.consume_skipping_ws()?;
            if bracket != b'[' {
                *state = State::Failed;
                return Err(serde::de::Error::custom("expected `[`"));
            }

            // Check for empty array
            let next_byte = reader.peek_skipping_ws()?;
            if next_byte == b']' {
                reader.consume()?; // Consume the closing bracket
                *state = State::Finished;
                return Ok(None);
            }

            // Not an empty array, deserialize the first element
            *state = State::AtMiddle;
            deserialize_single(reader).map(Some)
        }
        State::AtMiddle => {
            // At this point we've consumed the previous value and need to check for delimiter
            let delimiter = reader.consume_skipping_ws()?;
            match delimiter {
                b',' => deserialize_single(reader).map(Some),
                b']' => {
                    *state = State::Finished;
                    Ok(None)
                }
                _ => {
                    *state = State::Failed;
                    Err(serde::de::Error::custom("expected `,` or `]`"))
                }
            }
        }
        State::Finished | State::Failed => Ok(None),
    }
}

fn deserialize_single<T, R>(reader: &mut R) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
    R: io::Read,
{
    let next_obj = serde_json::Deserializer::from_reader(reader)
        .into_iter::<T>()
        .next();
    match next_obj {
        Some(result) => result,
        None => Err(serde::de::Error::custom("premature EOF")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Seek, SeekFrom};

    #[test]
    fn test_write_read_array() {
        let input: Vec<String> = vec!["1".into(), "5".into(), "0".into(), "2".into(), "9".into()];

        assert_write_and_read_results_equal(&input);
    }

    #[test]
    fn test_write_read_single_element_array() {
        let input: Vec<String> = vec!["1".into()];

        assert_write_and_read_results_equal(&input);
    }

    #[test]
    fn test_write_read_empty_array() {
        let input: Vec<String> = vec![];

        assert_write_and_read_results_equal(&input);
    }

    fn assert_write_and_read_results_equal<T>(input: &[T])
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + Clone,
    {
        // Create fake "file"
        let mut buffer = Cursor::new(Vec::new());
        serialize_seq(&mut buffer, input.iter().cloned()).unwrap();

        // Use the fake "file" as input
        buffer.seek(SeekFrom::Start(0)).unwrap();
        let result: Vec<T> = deserialize_seq(&mut buffer)
            .collect::<Result<_, serde_json::Error>>()
            .unwrap();

        assert_eq!(result, input.to_vec());
    }

    #[test]
    fn test_valid_json_reading() {
        let buffer = "[ 1 , 5 , 0 , 2 , 9 ]".as_bytes();

        let mut cursor = Cursor::new(buffer);
        let result: Vec<i32> = deserialize_seq(&mut cursor)
            .collect::<Result<_, serde_json::Error>>()
            .unwrap();

        assert_eq!(result, vec![1, 5, 0, 2, 9]);
    }

    #[test]
    fn test_invalid_json_reading() {
        let buffer = "[ \"key\": \"value\", ".as_bytes();

        let mut cursor = Cursor::new(buffer);
        let mut it = deserialize_seq::<String, &mut Cursor<&[u8]>>(&mut cursor);

        let first = it.next().unwrap();
        assert!(first.is_ok());

        let second = it.next().unwrap();
        assert!(second.is_err());
    }
}

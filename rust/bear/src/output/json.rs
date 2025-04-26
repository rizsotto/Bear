// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains functions to serialize and deserialize JSON arrays.
//!
//! The main objective is to provide a way to serialize and deserialize
//! entries from an iterator into a JSON array and vice versa. Iterators
//! allows us to process large datasets without loading everything into
//! memory at once.
//!
//! The format these methods are producing is a JSON array of objects.
//! It's *not* JSON lines format, which is a sequence of JSON objects
//! separated by newlines.

use std::io::{self, Read};

use serde::de::DeserializeOwned;
use serde::ser::{Serialize, SerializeSeq};
use serde::Serializer;
use serde_json::{Deserializer, Error, Result};

/// Serialize entries from an iterator into a JSON array.
pub fn write_array<W, T>(
    writer: W,
    entries: impl Iterator<Item = T>,
) -> std::result::Result<(), Error>
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
pub fn read_array<T, R>(mut reader: R) -> impl Iterator<Item = Result<T>>
where
    T: DeserializeOwned,
    R: io::Read,
{
    let mut at_start = State::AtStart;
    std::iter::from_fn(move || yield_next_obj(&mut reader, &mut at_start).transpose())
}

enum State {
    AtStart,
    AtMiddle,
    Finished,
    Failed,
}

fn yield_next_obj<T, R>(mut reader: R, state: &mut State) -> Result<Option<T>>
where
    T: DeserializeOwned,
    R: io::Read,
{
    match state {
        State::AtStart => match read_skipping_ws(&mut reader)? {
            b'[' => {
                let peek = read_skipping_ws(&mut reader)?;
                if peek == b']' {
                    *state = State::Finished;
                    Ok(None)
                } else {
                    *state = State::AtMiddle;
                    deserialize_single(io::Cursor::new([peek]).chain(reader)).map(Some)
                }
            }
            _ => {
                *state = State::Failed;
                Err(serde::de::Error::custom("expected `[`"))
            }
        },
        State::AtMiddle => match read_skipping_ws(&mut reader)? {
            b',' => deserialize_single(reader).map(Some),
            b']' => {
                *state = State::Finished;
                Ok(None)
            }
            _ => {
                *state = State::Failed;
                Err(serde::de::Error::custom("expected `,` or `]`"))
            }
        },
        State::Finished | State::Failed => Ok(None),
    }
}

fn deserialize_single<T, R>(reader: R) -> Result<T>
where
    T: DeserializeOwned,
    R: io::Read,
{
    let next_obj = Deserializer::from_reader(reader).into_iter::<T>().next();
    match next_obj {
        Some(result) => result,
        None => Err(serde::de::Error::custom("premature EOF")),
    }
}

fn read_skipping_ws(mut reader: impl io::Read) -> Result<u8> {
    loop {
        let mut byte = 0u8;
        reader
            .read_exact(std::slice::from_mut(&mut byte))
            .map_err(Error::io)?;

        if !byte.is_ascii_whitespace() {
            return Ok(byte);
        }
    }
}

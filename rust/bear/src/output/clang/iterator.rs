// SPDX-License-Identifier: GPL-3.0-or-later

//! Provides an iterator over a JSON array of objects.
//!
//! from https://github.com/serde-rs/json/issues/404#issuecomment-892957228

use std::io::{self, Read};

use serde::de::DeserializeOwned;
use serde_json::{Deserializer, Error, Result};

pub fn iter_json_array<T, R>(mut reader: R) -> impl Iterator<Item = Result<T>>
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

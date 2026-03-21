// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared serialization traits and error types for file formats.

use super::clang;
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

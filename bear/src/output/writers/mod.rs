// SPDX-License-Identifier: GPL-3.0-or-later

//! Output writer pipeline for compilation databases.
//!
//! This module provides a composable pipeline of writers that transform,
//! filter, and serialize compilation database entries. Each writer
//! implements the `IteratorWriter` trait and wraps an inner writer,
//! forming a chain of responsibility.

mod append;
mod atomic;
mod converter;
mod file;
mod source_filter;
mod unique;

use super::WriterError;

pub(crate) use append::AppendClangOutputWriter;
pub(crate) use atomic::AtomicClangOutputWriter;
pub(crate) use converter::ConverterClangOutputWriter;
pub(crate) use file::ClangOutputWriter;
pub(crate) use source_filter::SourceFilterOutputWriter;
pub(crate) use unique::UniqueOutputWriter;

/// A trait representing a writer for iterator type `T`.
///
/// This trait is implemented by types that can consume an iterator of type `T`
/// and write its elements to some output. The writing process may succeed or fail,
/// returning either `()` on success or an error.
pub(crate) trait IteratorWriter<T> {
    /// Writes the iterator as a sequence of elements.
    ///
    /// Consumes the iterator and returns either nothing on success or an error.
    fn write(self, items: impl Iterator<Item = T>) -> Result<(), WriterError>;
}

#[cfg(test)]
pub(crate) struct MockWriter;

#[cfg(test)]
impl IteratorWriter<crate::output::clang::Entry> for MockWriter {
    fn write(self, entries: impl Iterator<Item = crate::output::clang::Entry>) -> Result<(), WriterError> {
        // Consume the iterator to trigger upstream counting
        entries.for_each(drop);
        Ok(())
    }
}

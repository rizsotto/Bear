// SPDX-License-Identifier: GPL-3.0-or-later

//! Output writer pipeline for compilation databases.
//!
//! This module provides a composable pipeline of writers that transform,
//! filter, and serialize compilation database entries. Each writer
//! implements the `IteratorWriter` trait and wraps an inner writer,
//! forming a chain of responsibility.
//!
//! The [`create_pipeline`] function assembles the full writer stack from
//! configuration and returns an opaque writer that accepts semantic commands.

mod append;
mod atomic;
mod converter;
mod file;
mod source_filter;
mod unique;

use super::statistics::OutputStatistics;
use super::{WriterCreationError, WriterError};
use crate::{args, config, semantic};
use std::sync::Arc;

use append::AppendClangOutputWriter;
use atomic::AtomicClangOutputWriter;
use converter::ConverterClangOutputWriter;
use file::ClangOutputWriter;
use source_filter::SourceFilterOutputWriter;
use unique::UniqueOutputWriter;

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

/// The assembled writer pipeline type for Clang compilation databases.
type ClangWriterStack = ConverterClangOutputWriter<
    AppendClangOutputWriter<
        AtomicClangOutputWriter<SourceFilterOutputWriter<UniqueOutputWriter<ClangOutputWriter>>>,
    >,
>;

/// An opaque writer that accepts semantic commands and produces a compilation database.
///
/// This struct hides the concrete pipeline type from consumers. Use [`create_pipeline`]
/// to construct one.
pub(crate) struct SemanticCommandWriter {
    inner: ClangWriterStack,
}

impl SemanticCommandWriter {
    /// Writes semantic commands through the pipeline.
    pub(crate) fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        self.inner.write(semantics)
    }
}

/// Assembles the full output writer pipeline from configuration.
///
/// The pipeline processes semantic commands through the following stages:
/// 1. Convert semantic commands to compilation database entries
/// 2. Append entries from an existing database (if configured)
/// 3. Atomic file write (via temp file + rename)
/// 4. Source file path filtering
/// 5. Duplicate entry filtering
/// 6. Final file serialization
pub(crate) fn create_pipeline(
    args: &args::BuildSemantic,
    config: &config::Main,
    stats: Arc<OutputStatistics>,
) -> Result<SemanticCommandWriter, WriterCreationError> {
    let final_path = &args.path;
    let temp_path = &args.path.with_extension("tmp");

    let base_writer = ClangOutputWriter::create(temp_path, Arc::clone(&stats))?;
    let unique_writer =
        UniqueOutputWriter::create(base_writer, config.duplicates.clone(), Arc::clone(&stats))?;
    let source_filter_writer =
        SourceFilterOutputWriter::new(unique_writer, config.sources.clone(), Arc::clone(&stats));
    let atomic_writer = AtomicClangOutputWriter::new(source_filter_writer, temp_path, final_path);
    let append_writer =
        AppendClangOutputWriter::new(atomic_writer, final_path, args.append, Arc::clone(&stats));
    let formatted_writer = ConverterClangOutputWriter::new(append_writer, &config.format, Arc::clone(&stats));

    Ok(SemanticCommandWriter { inner: formatted_writer })
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

// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is responsible for writing the output of semantic analysis.
//!
//! The output is written as JSON compilation databases.
//! The module provides functionality to write these outputs to files,
//! handle duplicates, and format the output as needed.
//!
//! The `OutputWriter` struct represents the main entry point for writing output.
//! The input to the `OutputWriter` is a stream of `semantic::Command` instances.

pub mod clang;
mod formats;
mod json;
mod writers;

use crate::{args, config, semantic};
use thiserror::Error;
use writers::{
    AppendClangOutputWriter, AtomicClangOutputWriter, ClangOutputWriter,
    ConverterClangOutputWriter, IteratorWriter, UniqueOutputWriter,
};

// Re-export types for convenience.
pub use formats::{ExecutionEventDatabase, SerializationError, SerializationFormat};

/// A stack of output writers for Clang compilation databases.
type ClangWriterStack = ConverterClangOutputWriter<
    AppendClangOutputWriter<AtomicClangOutputWriter<UniqueOutputWriter<ClangOutputWriter>>>,
>;

/// Represents the output writer for JSON compilation databases.
///
/// The writer handles writing semantic analysis results as JSON compilation databases,
/// with support for deduplication, atomic writes, and appending to existing files.
pub struct OutputWriter {
    #[allow(private_interfaces)]
    writer: ClangWriterStack,
}

impl TryFrom<(&args::BuildSemantic, &config::Output)> for OutputWriter {
    type Error = WriterCreationError;

    fn try_from(value: (&args::BuildSemantic, &config::Output)) -> Result<Self, Self::Error> {
        let (args, config) = value;
        match config {
            config::Output::Clang {
                duplicates, format, ..
            } => {
                let final_path = &args.path;
                let temp_path = &args.path.with_extension("tmp");

                let base_writer = ClangOutputWriter::create(temp_path)?;
                let unique_writer = UniqueOutputWriter::create(base_writer, duplicates)?;
                let atomic_writer =
                    AtomicClangOutputWriter::new(unique_writer, temp_path, final_path);
                let append_writer =
                    AppendClangOutputWriter::new(atomic_writer, final_path, args.append);
                let formatted_writer = ConverterClangOutputWriter::new(append_writer, format)
                    .map_err(|e| WriterCreationError::Configuration(e.to_string()))?;

                Ok(Self {
                    writer: formatted_writer,
                })
            }
        }
    }
}

impl OutputWriter {
    /// Writes semantic commands using the configured output writer.
    ///
    /// # Arguments
    /// * `semantics` - An iterator of semantic commands to write
    ///
    /// # Returns
    /// `Ok(())` on success, or a `WriterError` if writing fails.
    pub fn write(
        self,
        semantics: impl Iterator<Item = semantic::Command>,
    ) -> Result<(), WriterError> {
        self.writer.write(semantics)
    }
}

/// Represents errors that can occur while creating an output writer.
#[derive(Error, Debug)]
pub enum WriterCreationError {
    #[error("Failed to create the output writer {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("Failed to configure the output writer: {0}")]
    Configuration(String),
}

/// Represents errors that can occur while writing output.
#[derive(Error, Debug)]
pub enum WriterError {
    #[error("Serialization error {0}: {1}")]
    Io(std::path::PathBuf, SerializationError),
}

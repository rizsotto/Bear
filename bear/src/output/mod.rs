// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is responsible for writing the output of the semantic analysis.
//!
//! The output can be in different formats, such as JSON compilation databases
//! or semantic analysis results in JSON format. The module provides functionality
//! to write these outputs to files, handle duplicates, and format the output
//! as needed.
//!
//! The `OutputWriter` enum represents the main entry point for writing the output.
//! The input of the `OutputWriter` is a stream of `semantic::CompilerCall` instances.

mod formats;
mod json;
mod writers;

use crate::{args, config, semantic};
use thiserror::Error;
use writers::{
    AppendClangOutputWriter, AtomicClangOutputWriter, ClangOutputWriter,
    ConverterClangOutputWriter, IteratorWriter, SemanticOutputWriter, UniqueOutputWriter,
};

// Re-export types for convenience.
pub use formats::{ExecutionEventDatabase, SerializationError, SerializationFormat};

/// Represents the output writer, which can handle different types of outputs.
///
/// This enum provides two variants:
/// - `Clang`: Writes output as a JSON compilation database.
/// - `Semantic`: Writes output as a JSON semantic analysis result.
///
/// The variants are selected at runtime based on the configuration provided.
pub enum OutputWriter {
    #[allow(private_interfaces)]
    Clang(
        ConverterClangOutputWriter<
            AppendClangOutputWriter<AtomicClangOutputWriter<UniqueOutputWriter<ClangOutputWriter>>>,
        >,
    ),
    #[allow(private_interfaces)]
    Semantic(SemanticOutputWriter),
}

impl TryFrom<(&args::BuildSemantic, &config::Output)> for OutputWriter {
    type Error = WriterCreationError;

    fn try_from(value: (&args::BuildSemantic, &config::Output)) -> Result<Self, Self::Error> {
        let (args, config) = value;
        match config {
            config::Output::Clang {
                duplicates, format, ..
            } => {
                let final_file_name = std::path::Path::new(&args.file_name);
                let temp_file_name = final_file_name.with_extension("tmp");

                let base_writer = ClangOutputWriter::create(&temp_file_name)?;
                let unique_writer = UniqueOutputWriter::create(base_writer, duplicates)?;
                let atomic_writer =
                    AtomicClangOutputWriter::new(unique_writer, &temp_file_name, final_file_name);
                let append_writer =
                    AppendClangOutputWriter::new(atomic_writer, final_file_name, args.append);
                let formatted_writer =
                    ConverterClangOutputWriter::new(append_writer, &format.entry);

                Ok(Self::Clang(formatted_writer))
            }
            config::Output::Semantic { .. } => {
                let path = std::path::Path::new(&args.file_name);
                let result = SemanticOutputWriter::try_from(path)?;
                Ok(Self::Semantic(result))
            }
        }
    }
}

impl OutputWriter {
    pub fn write(
        self,
        semantics: impl Iterator<Item = semantic::Command>,
    ) -> Result<(), WriterError> {
        match self {
            Self::Clang(writer) => writer.write(semantics),
            Self::Semantic(writer) => writer.write(semantics),
        }
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

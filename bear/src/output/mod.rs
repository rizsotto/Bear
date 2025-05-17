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

mod clang;
mod formats;
mod json;
mod writers;

use crate::{args, config, semantic};
use std::path;
use writers::{
    AppendClangOutputWriter, AtomicClangOutputWriter, ClangOutputWriter,
    ConverterClangOutputWriter, IteratorWriter, SemanticOutputWriter, UniqueOutputWriter,
};

// Re-export types for convenience.
pub use formats::{ExecutionEventDatabase, FileFormat};

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
    type Error = anyhow::Error;

    fn try_from(value: (&args::BuildSemantic, &config::Output)) -> Result<Self, Self::Error> {
        let (args, config) = value;
        match config {
            config::Output::Clang { duplicates, .. } => {
                let final_file_name = path::Path::new(&args.file_name);
                let temp_file_name = final_file_name.with_extension("tmp");

                let base_writer = ClangOutputWriter::create(&temp_file_name)?;
                let unique_writer = UniqueOutputWriter::create(base_writer, duplicates)?;
                let atomic_writer =
                    AtomicClangOutputWriter::new(unique_writer, &temp_file_name, final_file_name);
                let append_writer =
                    AppendClangOutputWriter::new(atomic_writer, args.append, final_file_name);
                let formatted_writer = ConverterClangOutputWriter::new(append_writer);

                Ok(Self::Clang(formatted_writer))
            }
            config::Output::Semantic { .. } => {
                let path = path::Path::new(&args.file_name);
                let result = SemanticOutputWriter::try_from(path)?;
                Ok(Self::Semantic(result))
            }
        }
    }
}

impl OutputWriter {
    pub(crate) fn write(
        self,
        semantics: impl Iterator<Item = semantic::CompilerCall>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Clang(writer) => writer.write(semantics),
            Self::Semantic(writer) => writer.write(semantics),
        }
    }
}

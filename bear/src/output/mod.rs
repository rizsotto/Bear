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

pub mod clang;
pub mod formats;
mod json;

use crate::{args, config, semantic};
use anyhow::Context;
use clang::{
    AppendClangOutputWriter, AtomicClangOutputWriter, ClangOutputWriter,
    FormattedClangOutputWriter, UniqueOutputWriter,
};
use formats::{FileFormat, JsonSemanticDatabase};
use std::{fs, io, path};

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
        FormattedClangOutputWriter<
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
                let formatted_writer = FormattedClangOutputWriter::new(append_writer);

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

/// The trait represents a writer for iterator type `T`.
///
/// This trait is implemented by types that can consume an iterator of type `T`
/// and write its elements to some output. The writing process may succeed or fail,
/// returning either `()` on success or an error.
pub trait IteratorWriter<T> {
    /// Writes the iterator as a sequence of elements.
    /// It consumes the iterator and returns either a nothing or an error.
    fn write(self, _: impl Iterator<Item = T>) -> anyhow::Result<()>;
}

/// This writer is used to write the semantic analysis results to a file.
///
/// # Note
/// The output format is not stable and may change in future versions.
/// It reflects the internal representation of the semantic analysis types.
struct SemanticOutputWriter {
    output: io::BufWriter<fs::File>,
}

impl TryFrom<&path::Path> for SemanticOutputWriter {
    type Error = anyhow::Error;

    fn try_from(file_name: &path::Path) -> Result<Self, Self::Error> {
        let output = fs::File::create(file_name)
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        Ok(Self { output })
    }
}

impl IteratorWriter<semantic::CompilerCall> for SemanticOutputWriter {
    fn write(self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        JsonSemanticDatabase::write(self.output, semantics)?;

        Ok(())
    }
}

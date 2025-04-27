// SPDX-License-Identifier: GPL-3.0-or-later

pub mod clang;
pub mod formats;
mod json;

use crate::{args, config, semantic};
use anyhow::Context;
use clang::{
    AppendClangOutputWriter, AtomicClangOutputWriter, ClangOutputWriter, FormattedClangOutputWriter,
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
            AppendClangOutputWriter<AtomicClangOutputWriter<ClangOutputWriter>>,
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

                let base_writer =
                    ClangOutputWriter::try_from((temp_file_name.as_path(), duplicates))?;
                let atomic_writer =
                    AtomicClangOutputWriter::new(base_writer, &temp_file_name, final_file_name);
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

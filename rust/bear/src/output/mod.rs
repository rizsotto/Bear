// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is responsible for writing the output of the semantic analysis.
//!
//! The output can be in different formats, such as JSON compilation databases
//! or semantic analysis results in JSON format. The module provides functionality
//! to write these outputs to files, handle duplicates, and format the output
//! as needed.
//!
//! The public API of this module includes the `IteratorWriter` trait, which is
//! implemented by different output writers. The `OutputWriter` enum represents
//! the main entry point for writing the output. The input of the `OutputWriter`
//! is a stream of `semantic::CompilerCall` instances.

pub mod clang;
pub mod filter_duplicates;
pub mod formatter;
mod json;

use super::{args, config, intercept, semantic};
use anyhow::Context;
use serde::ser::SerializeSeq;
use serde::Serializer;
use std::{fs, io, path};
use thiserror::Error;

/// The trait represents a file format that can be written to and read from.
///
/// The file format in this project is usually a sequence of values. This trait
/// provides a type-independent abstraction over the file format.
pub trait FileFormat<T> {
    fn write(self, _: impl Iterator<Item = T>) -> Result<(), Error>;
    fn read(self) -> impl Iterator<Item = Result<T, Error>>;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to open file: {0}")]
    IO(#[from] io::Error),
    #[error("Failed to serialize JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Format error: {0}")]
    Format(String),
}

/// The trait represents a JSON compilation database format.
///
/// The format is a JSON array format, which is a sequence of JSON objects
/// enclosed in square brackets. Each object represents a compilation
/// command.
/// 
/// # Note
/// The format itself is defined in the LLVM project documentation.
/// https://clang.llvm.org/docs/JSONCompilationDatabase.html
pub trait JsonCompilationDatabase: FileFormat<clang::Entry> {}

/// The trait represents a JSON semantic database format.
/// 
/// The format is a JSON array format, which is a sequence of JSON objects
/// enclosed in square brackets. Each object represents a semantic analysis
/// result.
///
/// # Note
/// The output format is not stable and may change in future versions.
pub trait JsonSemanticDatabase: FileFormat<semantic::CompilerCall> {}

/// The trait represents a database format for execution events.
///
/// The format is a JSON line format, which is a sequence of JSON objects
/// separated by newlines. https://jsonlines.org/
///
/// # Note
/// The output format is not stable and may change in future versions.
pub trait ExecutionEventDatabase: FileFormat<intercept::Event> {}

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

/// Represents the output writer, which can handle different types of outputs.
///
/// This enum provides two variants:
/// - `Clang`: Handles output for Clang compilation databases.
/// - `Semantic`: Handles output for semantic analysis results.
///
/// The specific behavior of each variant is implemented in their respective types.
pub enum OutputWriter {
    #[allow(private_interfaces)]
    Clang(FormattedClangOutputWriter),
    #[allow(private_interfaces)]
    Semantic(SemanticOutputWriter),
}

impl TryFrom<(&args::BuildSemantic, &config::Output)> for OutputWriter {
    type Error = anyhow::Error;

    fn try_from(value: (&args::BuildSemantic, &config::Output)) -> Result<Self, Self::Error> {
        let (args, config) = value;
        match config {
            config::Output::Clang { duplicates, .. } => {
                let result = FormattedClangOutputWriter::try_from((args, duplicates))?;
                Ok(Self::Clang(result))
            }
            config::Output::Semantic { .. } => {
                let path = path::Path::new(&args.file_name);
                let result = SemanticOutputWriter::try_from(path)?;
                Ok(Self::Semantic(result))
            }
        }
    }
}

impl IteratorWriter<semantic::CompilerCall> for OutputWriter {
    fn write(self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        match self {
            Self::Clang(writer) => writer.write(semantics),
            Self::Semantic(writer) => writer.write(semantics),
        }
    }
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
    // FIXME: this is the same method as `clang::write` for entries. Should be generalized?
    fn write(self, entries: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        let mut ser = serde_json::Serializer::pretty(self.output);
        let mut seq = ser.serialize_seq(None)?;
        for entry in entries {
            seq.serialize_element(&entry)?;
        }
        seq.end()?;

        Ok(())
    }
}

/// Formats `semantic::CompilerCall` instances into `clang::Entry` objects.
struct FormattedClangOutputWriter {
    formatter: formatter::EntryFormatter,
    writer: AppendClangOutputWriter,
}

impl TryFrom<(&args::BuildSemantic, &config::DuplicateFilter)> for FormattedClangOutputWriter {
    type Error = anyhow::Error;

    fn try_from(
        value: (&args::BuildSemantic, &config::DuplicateFilter),
    ) -> Result<Self, Self::Error> {
        let (args, config) = value;

        let formatter = formatter::EntryFormatter::new();
        let writer = AppendClangOutputWriter::try_from((args, config))?;

        Ok(Self { formatter, writer })
    }
}

impl IteratorWriter<semantic::CompilerCall> for FormattedClangOutputWriter {
    fn write(self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        let entries = semantics.flat_map(|semantic| self.formatter.apply(semantic));
        self.writer.write(entries)
    }
}

/// Handles the logic for appending entries to an existing Clang output file.
///
/// This writer supports reading existing entries from a compilation database file,
/// combining them with new entries, and writing the result back to the file.
/// If the file does not exist and the append option is enabled, it logs a warning
/// and writes only the new entries.
struct AppendClangOutputWriter {
    writer: AtomicClangOutputWriter,
    path: Option<path::PathBuf>,
}

impl TryFrom<(&args::BuildSemantic, &config::DuplicateFilter)> for AppendClangOutputWriter {
    type Error = anyhow::Error;

    fn try_from(
        value: (&args::BuildSemantic, &config::DuplicateFilter),
    ) -> Result<Self, Self::Error> {
        let (args, config) = value;

        let file_name = path::Path::new(&args.file_name);
        let path = if file_name.exists() {
            Some(file_name.to_path_buf())
        } else {
            if args.append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            None
        };

        let writer = AtomicClangOutputWriter::try_from((file_name, config))?;

        Ok(Self { writer, path })
    }
}

impl IteratorWriter<clang::Entry> for AppendClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> anyhow::Result<()> {
        if let Some(path) = self.path {
            let entries_from_db = Self::read_from_compilation_db(&path)?;
            let final_entries = entries_from_db.chain(entries);
            self.writer.write(final_entries)
        } else {
            self.writer.write(entries)
        }
    }
}

impl AppendClangOutputWriter {
    /// Reads the compilation database from a file.
    ///
    /// NOTE: The function is intentionally not getting any `&self` reference,
    /// because the logic is not bound to the instance.
    fn read_from_compilation_db(
        source: &path::Path,
    ) -> anyhow::Result<impl Iterator<Item = clang::Entry>> {
        let source_copy = source.to_path_buf();

        let file = fs::File::open(source)
            .map(io::BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = clang::read(file).filter_map(move |candidate| match candidate {
            Ok(entry) => Some(entry),
            Err(error) => {
                log::error!("Failed to read file: {:?}, reason: {}", source_copy, error);
                None
            }
        });
        Ok(entries)
    }
}

/// Responsible for writing a JSON compilation database file atomically.
///
/// The file is first written to a temporary file and then renamed to the final file name.
/// This ensures that the output file is not left in an inconsistent state in case of errors.
struct AtomicClangOutputWriter {
    writer: ClangOutputWriter,
    temp_file_name: path::PathBuf,
    final_file_name: path::PathBuf,
}

impl TryFrom<(&path::Path, &config::DuplicateFilter)> for AtomicClangOutputWriter {
    type Error = anyhow::Error;

    fn try_from(value: (&path::Path, &config::DuplicateFilter)) -> Result<Self, Self::Error> {
        let (file_name, config) = value;

        let temp_file_name = file_name.with_extension("tmp");
        let writer = ClangOutputWriter::try_from((temp_file_name.as_path(), config))?;

        Ok(Self {
            writer,
            temp_file_name: temp_file_name.to_path_buf(),
            final_file_name: file_name.to_path_buf(),
        })
    }
}

impl IteratorWriter<clang::Entry> for AtomicClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> anyhow::Result<()> {
        let temp_file_name = self.temp_file_name.clone();
        let final_file_name = self.final_file_name.clone();

        self.writer.write(entries)?;

        fs::rename(&temp_file_name, &final_file_name).with_context(|| {
            format!(
                "Failed to rename file from '{:?}' to '{:?}'.",
                temp_file_name, final_file_name
            )
        })?;

        Ok(())
    }
}

/// Responsible for writing a JSON compilation database file from the given entries.
///
/// # Features
/// - Writes the entries to a file.
/// - Filters duplicates based on the provided configuration.
struct ClangOutputWriter {
    output: io::BufWriter<fs::File>,
    filter: filter_duplicates::DuplicateFilter,
}

impl TryFrom<(&path::Path, &config::DuplicateFilter)> for ClangOutputWriter {
    type Error = anyhow::Error;

    fn try_from(value: (&path::Path, &config::DuplicateFilter)) -> Result<Self, Self::Error> {
        let (file_name, config) = value;

        let output = fs::File::create(file_name)
            .map(io::BufWriter::new)
            .with_context(|| format!("Failed to open file: {:?}", file_name))?;

        let filter = filter_duplicates::DuplicateFilter::try_from(config.clone())?;

        Ok(Self { output, filter })
    }
}

impl IteratorWriter<clang::Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> anyhow::Result<()> {
        let mut filter = self.filter.clone();
        let filtered_entries = entries.filter(move |entry| filter.unique(entry));
        clang::write(self.output, filtered_entries)?;
        Ok(())
    }
}

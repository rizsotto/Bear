// SPDX-License-Identifier: GPL-3.0-or-later

//! This crate provides support for reading and writing JSON compilation database files.
//!
//! A compilation database is a set of records which describe the compilation of the
//! source files in a given project. It describes the compiler invocation command to
//! compile a source module to an object file.
//!
//! This database can have many forms. One well known and supported format is the JSON
//! compilation database, which is a simple JSON file having the list of compilation
//! as an array. The definition of the JSON compilation database files is done in the
//! LLVM project [documentation](https://clang.llvm.org/docs/JSONCompilationDatabase.html).

mod filter_duplicates;
mod formatter;
mod type_de;

use super::formats::{FileFormat, JsonCompilationDatabase};
use super::IteratorWriter;
use crate::{config, semantic};
use anyhow::Context;
use serde::Serialize;
use std::{fs, io, path};

/// Represents an entry of the compilation database.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Entry {
    /// The main translation unit source processed by this compilation step.
    /// This is used by tools as the key into the compilation database.
    /// There can be multiple command objects for the same file, for example if the same
    /// source file is compiled with different configurations.
    pub file: path::PathBuf,
    /// The compile command executed. This must be a valid command to rerun the exact
    /// compilation step for the translation unit in the environment the build system uses.
    /// Shell expansion is not supported.
    pub arguments: Vec<String>,
    /// The working directory of the compilation. All paths specified in the command or
    /// file fields must be either absolute or relative to this directory.
    pub directory: path::PathBuf,
    /// The name of the output created by this compilation step. This field is optional.
    /// It can be used to distinguish different processing modes of the same input file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<path::PathBuf>,
}

#[cfg(test)]
pub fn entry(file: &str, arguments: Vec<&str>, directory: &str, output: Option<&str>) -> Entry {
    Entry {
        file: path::PathBuf::from(file),
        arguments: arguments.into_iter().map(String::from).collect(),
        directory: path::PathBuf::from(directory),
        output: output.map(path::PathBuf::from),
    }
}

/// Formats `semantic::CompilerCall` instances into `Entry` objects.
pub(super) struct FormattedClangOutputWriter<T: IteratorWriter<Entry>> {
    formatter: formatter::EntryFormatter,
    writer: T,
}

impl<T: IteratorWriter<Entry>> FormattedClangOutputWriter<T> {
    pub(super) fn new(writer: T) -> Self {
        let formatter = formatter::EntryFormatter::new();
        Self { formatter, writer }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<semantic::CompilerCall>
    for FormattedClangOutputWriter<T>
{
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
pub(super) struct AppendClangOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    path: Option<path::PathBuf>,
}

impl<T: IteratorWriter<Entry>> AppendClangOutputWriter<T> {
    pub(super) fn new(writer: T, append: bool, file_name: &path::Path) -> Self {
        let path = if file_name.exists() {
            Some(file_name.to_path_buf())
        } else {
            if append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            None
        };
        Self { writer, path }
    }

    /// Reads the compilation database from a file.
    ///
    /// NOTE: The function is intentionally not getting any `&self` reference,
    /// because the logic is not bound to the instance.
    fn read_from_compilation_db(
        source: &path::Path,
    ) -> anyhow::Result<impl Iterator<Item = Entry>> {
        let source_copy = source.to_path_buf();

        let file = fs::File::open(source)
            .map(io::BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = JsonCompilationDatabase::read_and_ignore(file, source_copy);
        Ok(entries)
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for AppendClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        if let Some(path) = self.path {
            let entries_from_db = Self::read_from_compilation_db(&path)?;
            let final_entries = entries_from_db.chain(entries);
            self.writer.write(final_entries)
        } else {
            self.writer.write(entries)
        }
    }
}

/// Responsible for writing a JSON compilation database file atomically.
///
/// The file is first written to a temporary file and then renamed to the final file name.
/// This ensures that the output file is not left in an inconsistent state in case of errors.
pub(super) struct AtomicClangOutputWriter<T: IteratorWriter<Entry>> {
    writer: T,
    temp_file_name: path::PathBuf,
    final_file_name: path::PathBuf,
}

impl<T: IteratorWriter<Entry>> AtomicClangOutputWriter<T> {
    pub(super) fn new(
        writer: T,
        temp_file_name: &path::Path,
        final_file_name: &path::Path,
    ) -> Self {
        Self {
            writer,
            temp_file_name: temp_file_name.to_path_buf(),
            final_file_name: final_file_name.to_path_buf(),
        }
    }
}

impl<T: IteratorWriter<Entry>> IteratorWriter<Entry> for AtomicClangOutputWriter<T> {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
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
pub(super) struct ClangOutputWriter {
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

impl IteratorWriter<Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = Entry>) -> anyhow::Result<()> {
        let mut filter = self.filter.clone();
        let filtered_entries = entries.filter(move |entry| filter.unique(entry));
        JsonCompilationDatabase::write(self.output, filtered_entries)?;
        Ok(())
    }
}

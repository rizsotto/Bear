// SPDX-License-Identifier: GPL-3.0-or-later

pub mod clang;
pub mod filter_duplicates;
pub mod formats;
pub mod formatter;
mod json;

use crate::{args, config, semantic};
use anyhow::Context;
use formats::{FileFormat, JsonCompilationDatabase, JsonSemanticDatabase};
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

/// Formats `semantic::CompilerCall` instances into `clang::Entry` objects.
struct FormattedClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    formatter: formatter::EntryFormatter,
    writer: T,
}

impl<T: IteratorWriter<clang::Entry>> FormattedClangOutputWriter<T> {
    fn new(writer: T) -> Self {
        let formatter = formatter::EntryFormatter::new();
        Self { formatter, writer }
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<semantic::CompilerCall>
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
struct AppendClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    path: Option<path::PathBuf>,
}

impl<T: IteratorWriter<clang::Entry>> AppendClangOutputWriter<T> {
    fn new(writer: T, append: bool, file_name: &path::Path) -> Self {
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
    ) -> anyhow::Result<impl Iterator<Item = clang::Entry>> {
        let source_copy = source.to_path_buf();

        let file = fs::File::open(source)
            .map(io::BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = JsonCompilationDatabase::read_and_ignore(file, source_copy);
        Ok(entries)
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for AppendClangOutputWriter<T> {
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

/// Responsible for writing a JSON compilation database file atomically.
///
/// The file is first written to a temporary file and then renamed to the final file name.
/// This ensures that the output file is not left in an inconsistent state in case of errors.
struct AtomicClangOutputWriter<T: IteratorWriter<clang::Entry>> {
    writer: T,
    temp_file_name: path::PathBuf,
    final_file_name: path::PathBuf,
}

impl<T: IteratorWriter<clang::Entry>> AtomicClangOutputWriter<T> {
    fn new(writer: T, temp_file_name: &path::Path, final_file_name: &path::Path) -> Self {
        Self {
            writer,
            temp_file_name: temp_file_name.to_path_buf(),
            final_file_name: final_file_name.to_path_buf(),
        }
    }
}

impl<T: IteratorWriter<clang::Entry>> IteratorWriter<clang::Entry> for AtomicClangOutputWriter<T> {
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
        JsonCompilationDatabase::write(self.output, filtered_entries)?;
        Ok(())
    }
}

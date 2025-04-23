// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::Event;
use crate::output::OutputWriter;
use crate::semantic::interpreters::create_interpreter;
use crate::semantic::transformation;
use crate::semantic::transformation::FilterAndFormat;
use crate::{args, config, output, semantic};
use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// The semantic analysis that is independent of the event source.
pub(super) struct SemanticAnalysisPipeline {
    interpreter: Box<dyn semantic::Interpreter>,
    transformation: Box<dyn transformation::Transformation>,
    output_writer: OutputWriterImpl,
}

impl SemanticAnalysisPipeline {
    /// Create a new semantic mode instance.
    pub(super) fn from(output: args::BuildSemantic, config: &config::Main) -> anyhow::Result<Self> {
        let interpreter = create_interpreter(config);
        let transformation = FilterAndFormat::try_from(&config.output)?;
        let output_writer = OutputWriterImpl::create(&output, &config.output)?;

        Ok(Self {
            interpreter: Box::new(interpreter),
            transformation: Box::new(transformation),
            output_writer,
        })
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis.
    pub(super) fn analyze_and_write(
        self,
        events: impl IntoIterator<Item = Event>,
    ) -> anyhow::Result<()> {
        // Set up the pipeline of compilation database entries.
        let semantics = events
            .into_iter()
            .inspect(|event| log::debug!("event: {}", event))
            .flat_map(|event| self.interpreter.recognize(&event.execution))
            .inspect(|semantic| log::debug!("semantic: {:?}", semantic))
            .flat_map(|semantic| self.transformation.apply(semantic))
            .inspect(|semantic| log::debug!("transformed: {:?}", semantic));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        self.output_writer.run(semantics)
    }
}

/// The output writer implementation.
///
/// This is a workaround for the lack of trait object support for generic arguments.
/// https://doc.rust-lang.org/reference/items/traits.html#object-safety.
pub(crate) enum OutputWriterImpl {
    Clang(ClangOutputWriter),
    Semantic(SemanticOutputWriter),
}

impl OutputWriter for OutputWriterImpl {
    fn run(&self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        match self {
            OutputWriterImpl::Clang(writer) => writer.run(semantics),
            OutputWriterImpl::Semantic(writer) => writer.run(semantics),
        }
    }
}

impl OutputWriterImpl {
    /// Create a new instance of the output writer.
    pub(crate) fn create(
        args: &args::BuildSemantic,
        config: &config::Output,
    ) -> anyhow::Result<OutputWriterImpl> {
        // TODO: This method should fail early if the output file is not writable.
        match config {
            config::Output::Clang { duplicates, .. } => {
                let result = ClangOutputWriter {
                    output: PathBuf::from(&args.file_name),
                    append: args.append,
                    formatter: output::formatter::EntryFormatter::new(),
                    duplicate_filter: duplicates.clone(),
                };
                Ok(OutputWriterImpl::Clang(result))
            }
            config::Output::Semantic { .. } => {
                let result = SemanticOutputWriter {
                    output: PathBuf::from(&args.file_name),
                };
                Ok(OutputWriterImpl::Semantic(result))
            }
        }
    }
}

pub(crate) struct SemanticOutputWriter {
    output: PathBuf,
}

impl OutputWriter for SemanticOutputWriter {
    fn run(&self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        let file_name = &self.output;
        let file = File::create(file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;

        semantic::serialize(file, semantics)?;

        Ok(())
    }
}

/// Responsible for writing the final compilation database file.
///
/// Implements filtering, formatting and atomic file writing.
/// (Atomic file writing implemented by writing to a temporary file and renaming it.)
///
/// Filtering is implemented by the `filter` module, and the formatting is implemented by the
/// `json_compilation_db` module.
pub(crate) struct ClangOutputWriter {
    output: PathBuf,
    append: bool,
    formatter: output::formatter::EntryFormatter,
    duplicate_filter: config::DuplicateFilter,
}

impl OutputWriter for ClangOutputWriter {
    /// Implements the main logic of the output writer.
    fn run(&self, semantics: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        let entries = semantics.flat_map(|semantic| self.formatter.apply(semantic));
        if self.append && self.output.exists() {
            let entries_from_db = Self::read_from_compilation_db(self.output.as_path())?;
            let final_entries = entries.chain(entries_from_db);
            self.write_into_compilation_db(final_entries)
        } else {
            if self.append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            self.write_into_compilation_db(entries)
        }
    }
}

impl ClangOutputWriter {
    /// Write the entries to the compilation database.
    ///
    /// The entries are written to a temporary file and then renamed to the final output.
    /// This guaranties that the output file is always in a consistent state.
    fn write_into_compilation_db(
        &self,
        entries: impl Iterator<Item = output::clang::Entry>,
    ) -> anyhow::Result<()> {
        // Filter out the entries as per the configuration.
        let mut duplicate_filter: output::filter::EntryPredicate =
            From::from(&self.duplicate_filter);
        let filtered_entries = entries.filter(move |entry| duplicate_filter(entry));
        // Write the entries to a temporary file.
        self.write_into_temporary_compilation_db(filtered_entries)
            .and_then(|temp| {
                // Rename the temporary file to the final output.
                std::fs::rename(temp.as_path(), self.output.as_path()).with_context(|| {
                    format!(
                        "Failed to rename file from '{:?}' to '{:?}'.",
                        temp.as_path(),
                        self.output.as_path()
                    )
                })
            })
    }

    /// Write the entries to a temporary file and returns the temporary file name.
    fn write_into_temporary_compilation_db(
        &self,
        entries: impl Iterator<Item = output::clang::Entry>,
    ) -> anyhow::Result<PathBuf> {
        // Generate a temporary file name.
        let file_name = self.output.with_extension("tmp");
        // Open the file for writing.
        let file = File::create(&file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;
        // Write the entries to the file.
        output::clang::write(file, entries)
            .with_context(|| format!("Failed to write entries: {:?}", file_name.as_path()))?;
        // Return the temporary file name.
        Ok(file_name)
    }

    /// Read the compilation database from a file.
    fn read_from_compilation_db(
        source: &Path,
    ) -> anyhow::Result<impl Iterator<Item = output::clang::Entry>> {
        let source_copy = source.to_path_buf();

        let file = OpenOptions::new()
            .read(true)
            .open(source)
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = output::clang::read(file).filter_map(move |candidate| match candidate {
            Ok(entry) => Some(entry),
            Err(error) => {
                log::error!("Failed to read file: {:?}, reason: {}", source_copy, error);
                None
            }
        });
        Ok(entries)
    }
}

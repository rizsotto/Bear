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
mod filtering;
mod stdout;
mod synthesis;
mod validating;

use super::statistics::OutputStatistics;
use super::{WriterCreationError, WriterError};
use crate::args::is_stdio;
use crate::{args, config};
use std::sync::Arc;

use append::AppendClangOutputWriter;
use atomic::AtomicClangOutputWriter;
use converter::ConverterClangOutputWriter;
use file::ClangOutputWriter;
use filtering::{DuplicateEntryFilter, FilteredOutputWriter, SourceEntryFilter};
use stdout::ClangStdoutOutputWriter;
use synthesis::HeaderEntrySynthesizer;
use validating::ValidatingOutputWriter;

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

/// The shared validating/deduplicating/synthesizing/source-filtering stack
/// that sits between a base writer and the final format converter. Both the
/// file pipeline and the stdout pipeline build this identically, so
/// filtering and formatting behave the same regardless of the sink.
type FilteredStack<Base> = FilteredOutputWriter<
    HeaderEntrySynthesizer<FilteredOutputWriter<ValidatingOutputWriter<Base>, DuplicateEntryFilter>>,
    SourceEntryFilter,
>;

/// The assembled writer pipeline type for Clang compilation databases
/// written to a file: atomic temp-file write, then optional append.
type ClangWriterStack = ConverterClangOutputWriter<
    AppendClangOutputWriter<AtomicClangOutputWriter<FilteredStack<ClangOutputWriter>>>,
>;

/// The assembled writer pipeline type for Clang compilation databases
/// streamed to standard output: no atomic write and no append, since
/// neither is possible on a stream.
type ClangWriterStdoutStack = ConverterClangOutputWriter<FilteredStack<ClangStdoutOutputWriter>>;

/// The concrete pipeline behind [`SemanticCommandWriter`].
///
/// A plain trait object is not an option here: `IteratorWriter::write` takes
/// a generic `impl Iterator` parameter, which makes the trait non-object-safe.
/// An enum over the two concrete pipeline shapes is the direct alternative.
enum Pipeline {
    File(ClangWriterStack),
    Stdout(ClangWriterStdoutStack),
}

/// An opaque writer that accepts semantic commands and produces a compilation database.
///
/// This struct hides the concrete pipeline type from consumers. Use [`create_pipeline`]
/// to construct one.
pub(crate) struct SemanticCommandWriter {
    inner: Pipeline,
}

impl SemanticCommandWriter {
    /// Writes semantic commands through the pipeline.
    pub(crate) fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        match self.inner {
            Pipeline::File(writer) => writer.write(semantics),
            Pipeline::Stdout(writer) => writer.write(semantics),
        }
    }
}

/// Builds the shared validating/deduplicating/synthesizing/source-filtering
/// stack on top of `base_writer`. Used by both the file and stdout branches
/// of [`create_pipeline`] so dedup, header synthesis, and source filtering
/// behave identically regardless of the sink.
fn build_filtered_stack<Base: IteratorWriter<crate::output::clang::Entry>>(
    base_writer: Base,
    config: &config::Main,
    stats: Arc<OutputStatistics>,
) -> Result<FilteredStack<Base>, WriterCreationError> {
    let validating_writer = ValidatingOutputWriter::new(base_writer, Arc::clone(&stats));
    let duplicate_filter = DuplicateEntryFilter::try_from(config.duplicates.clone())
        .map_err(|err| WriterCreationError::Configuration(err.to_string()))?;
    let unique_writer =
        FilteredOutputWriter::new(validating_writer, duplicate_filter, Arc::clone(&stats), |s| {
            &s.duplicates_detected
        });
    let synthesizer = HeaderEntrySynthesizer::new(unique_writer, config.headers.clone(), Arc::clone(&stats));
    let source_filter_writer = FilteredOutputWriter::new(
        synthesizer,
        SourceEntryFilter::from(config.sources.clone()),
        Arc::clone(&stats),
        |s| &s.entries_filtered_by_source,
    );

    Ok(source_filter_writer)
}

/// Assembles the full output writer pipeline from configuration.
///
/// When `args.path` is the `-` sentinel, the pipeline streams the
/// compilation database to standard output instead of a file: the atomic
/// temp-file write and the append step are skipped, since neither applies
/// to a stream (see [`ClangWriterStdoutStack`]). `--append` combined with
/// `-` is rejected up front, since appending to a stream is impossible.
///
/// Otherwise the pipeline processes semantic commands through the following
/// stages:
/// 1. Convert semantic commands to compilation database entries
/// 2. Append entries from an existing database (if configured)
/// 3. Atomic file write (via temp file + rename)
/// 4. Source file path filtering
/// 5. Header entry synthesis (if configured; clones a donor translation
///    unit's arguments onto sibling header files)
/// 6. Duplicate entry filtering
/// 7. Entry validation (drop invalid entries with a warning; earlier filters
///    never see an entry that will be dropped here)
/// 8. Final file serialization
pub(crate) fn create_pipeline(
    args: &args::BuildSemantic,
    config: &config::Main,
    stats: Arc<OutputStatistics>,
) -> Result<SemanticCommandWriter, WriterCreationError> {
    if is_stdio(&args.path) {
        if args.append {
            return Err(WriterCreationError::Configuration("cannot append to standard output".to_string()));
        }

        let base_writer = ClangStdoutOutputWriter::new(Arc::clone(&stats));
        let source_filter_writer = build_filtered_stack(base_writer, config, Arc::clone(&stats))?;
        let formatted_writer =
            ConverterClangOutputWriter::new(source_filter_writer, &config.format, Arc::clone(&stats));

        return Ok(SemanticCommandWriter { inner: Pipeline::Stdout(formatted_writer) });
    }

    let final_path = &args.path;
    let temp_path = &args.path.with_extension("tmp");

    let base_writer = ClangOutputWriter::create(temp_path, Arc::clone(&stats))?;
    let source_filter_writer = build_filtered_stack(base_writer, config, Arc::clone(&stats))?;
    let atomic_writer = AtomicClangOutputWriter::new(source_filter_writer, temp_path, final_path);
    let append_writer =
        AppendClangOutputWriter::new(atomic_writer, final_path, args.append, Arc::clone(&stats));
    let formatted_writer = ConverterClangOutputWriter::new(append_writer, &config.format, Arc::clone(&stats));

    Ok(SemanticCommandWriter { inner: Pipeline::File(formatted_writer) })
}

#[cfg(test)]
mod fixtures {
    use super::{IteratorWriter, WriterError};

    /// A test writer that collects all entries into a shared vector.
    ///
    /// This allows tests to verify exactly which entries pass through a writer
    /// pipeline, including their contents and ordering. When inspection is not
    /// needed, simply ignore the collected vector.
    pub(crate) struct CollectingWriter {
        pub collected: std::sync::Arc<std::sync::Mutex<Vec<crate::output::clang::Entry>>>,
    }

    impl CollectingWriter {
        pub fn new() -> (Self, std::sync::Arc<std::sync::Mutex<Vec<crate::output::clang::Entry>>>) {
            let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            (Self { collected: std::sync::Arc::clone(&collected) }, collected)
        }
    }

    impl IteratorWriter<crate::output::clang::Entry> for CollectingWriter {
        fn write(
            self,
            entries: impl Iterator<Item = crate::output::clang::Entry>,
        ) -> Result<(), WriterError> {
            let mut collected = self.collected.lock().unwrap();
            collected.extend(entries);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use semantic::{ArgumentKind, Command, CompilerPass, PassEffect};
    use std::sync::atomic::Ordering;

    fn make_compile_command(file: &str) -> semantic::Command {
        Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Compiler, vec!["/usr/bin/gcc"]),
                (ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)), vec!["-c"]),
                (ArgumentKind::Source { binary: false }, vec![file]),
            ],
        )
    }

    #[test]
    fn test_create_pipeline_writes_entries() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("compile_commands.json");
        let config = config::Main::default();
        let args = args::BuildSemantic { path: output_path.clone(), append: false };
        let stats = OutputStatistics::new();

        let pipeline = create_pipeline(&args, &config, Arc::clone(&stats)).unwrap();

        let commands = vec![make_compile_command("file1.c"), make_compile_command("file2.c")];

        pipeline.write(commands.into_iter()).unwrap();

        assert!(output_path.exists());
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("file1.c"));
        assert!(content.contains("file2.c"));

        assert_eq!(stats.compilation_entries_produced.load(Ordering::Relaxed), 2);
        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 2);
        assert_eq!(stats.duplicates_detected.load(Ordering::Relaxed), 0);
        assert_eq!(stats.entries_filtered_by_source.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_create_pipeline_deduplicates_entries() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("compile_commands.json");
        let config = config::Main::default();
        let args = args::BuildSemantic { path: output_path.clone(), append: false };
        let stats = OutputStatistics::new();

        let pipeline = create_pipeline(&args, &config, Arc::clone(&stats)).unwrap();

        let commands = vec![
            make_compile_command("file1.c"),
            make_compile_command("file1.c"),
            make_compile_command("file2.c"),
        ];

        pipeline.write(commands.into_iter()).unwrap();

        assert_eq!(stats.compilation_entries_produced.load(Ordering::Relaxed), 3);
        assert_eq!(stats.duplicates_detected.load(Ordering::Relaxed), 1);
        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_create_pipeline_filters_by_source() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("compile_commands.json");
        let config = config::Main {
            sources: config::SourceFilter {
                directories: vec![config::DirectoryRule {
                    path: std::path::PathBuf::from("/usr/include"),
                    action: config::DirectoryAction::Exclude,
                }],
                files: vec![],
            },
            ..config::Main::default()
        };
        let args = args::BuildSemantic { path: output_path.clone(), append: false };
        let stats = OutputStatistics::new();

        let pipeline = create_pipeline(&args, &config, Arc::clone(&stats)).unwrap();

        let commands = vec![make_compile_command("src/main.c"), make_compile_command("/usr/include/stdio.h")];

        pipeline.write(commands.into_iter()).unwrap();

        assert_eq!(stats.entries_filtered_by_source.load(Ordering::Relaxed), 1);
        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 1);

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("src/main.c"));
        assert!(!content.contains("stdio.h"));
    }

    #[test]
    fn test_create_pipeline_empty_input() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("compile_commands.json");
        let config = config::Main::default();
        let args = args::BuildSemantic { path: output_path.clone(), append: false };
        let stats = OutputStatistics::new();

        let pipeline = create_pipeline(&args, &config, Arc::clone(&stats)).unwrap();
        pipeline.write(std::iter::empty()).unwrap();

        assert!(output_path.exists());
        assert_eq!(stats.semantic_commands_received.load(Ordering::Relaxed), 0);
        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 0);
    }
}

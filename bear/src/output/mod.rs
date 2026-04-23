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
mod intercept;
mod statistics;
mod writers;

use crate::{args, config, semantic};
use std::sync::Arc;
use thiserror::Error;

// Re-export types for convenience.
pub use formats::{SerializationError, SerializationFormat};
pub use intercept::ExecutionEventDatabase;
pub use statistics::OutputStatistics;

/// Represents the output writer for JSON compilation databases.
///
/// The writer handles writing semantic analysis results as JSON compilation databases,
/// with support for deduplication, atomic writes, and appending to existing files.
pub struct OutputWriter {
    writer: writers::SemanticCommandWriter,
    stats: Arc<OutputStatistics>,
}

impl TryFrom<(&args::BuildSemantic, &config::Main)> for OutputWriter {
    type Error = WriterCreationError;

    fn try_from(value: (&args::BuildSemantic, &config::Main)) -> Result<Self, Self::Error> {
        let (args, config) = value;
        let stats = OutputStatistics::new();
        let writer = writers::create_pipeline(args, config, Arc::clone(&stats))?;

        Ok(Self { writer, stats })
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
    pub fn write(self, semantics: impl Iterator<Item = semantic::Command>) -> Result<(), WriterError> {
        let result = self.writer.write(semantics);

        // Log pipeline statistics
        log::info!("{}", self.stats);

        // If every candidate entry was dropped by validation, surface that
        // at ERROR level so the empty compilation database is never silent.
        // The per-entry WARN logs from the validator explain individual
        // drops; this one explains the shape of the whole output.
        let written = self.stats.entries_written.load(std::sync::atomic::Ordering::Relaxed);
        let dropped = self.stats.entries_dropped_invalid.load(std::sync::atomic::Ordering::Relaxed);
        if written == 0 && dropped > 0 {
            log::error!(
                "Compilation database is empty: all {dropped} candidate entries were dropped due to validation failures; see WARN log lines above for per-entry reasons",
            );
        }

        result
    }

    /// Returns the statistics collected during the output pipeline execution.
    pub fn statistics(&self) -> &Arc<OutputStatistics> {
        &self.stats
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{ArgumentKind, Command, CompilerPass, PassEffect};
    use std::sync::atomic::Ordering;

    fn make_compile_command(file: &str) -> Command {
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
    fn test_output_writer_try_from_and_write() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("compile_commands.json");
        let args = args::BuildSemantic { path: output_path.clone(), append: false };
        let config = config::Main::default();

        let writer = OutputWriter::try_from((&args, &config)).unwrap();

        let stats = writer.statistics().clone();

        let commands = vec![make_compile_command("main.c"), make_compile_command("util.c")];

        writer.write(commands.into_iter()).unwrap();

        assert!(output_path.exists());
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("main.c"));
        assert!(content.contains("util.c"));

        assert_eq!(stats.entries_written.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_output_writer_creation_failure() {
        let args = args::BuildSemantic {
            path: std::path::PathBuf::from("/nonexistent/dir/output.json"),
            append: false,
        };
        let config = config::Main::default();

        let result = OutputWriter::try_from((&args, &config));
        assert!(result.is_err());
    }
}

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
mod json;
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

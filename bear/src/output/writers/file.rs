// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use crate::output::clang;
use crate::output::clang::serialization::JsonCompilationDatabase;
use crate::output::formats::SerializationFormat;
use crate::output::statistics::OutputStatistics;
use crate::output::{WriterCreationError, WriterError};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{fs, io, path};

/// The type represents a writer that writes JSON compilation database files from given entries.
///
/// # Features
/// - Writes the entries to a file.
/// - Formats the entries to the configured shape.
pub(crate) struct ClangOutputWriter {
    output: io::BufWriter<fs::File>,
    path: path::PathBuf,
    stats: Arc<OutputStatistics>,
}

impl ClangOutputWriter {
    pub(crate) fn create(
        path: &path::Path,
        stats: Arc<OutputStatistics>,
    ) -> Result<Self, WriterCreationError> {
        let output = fs::File::create(path)
            .map(io::BufWriter::new)
            .map_err(|err| WriterCreationError::Io(path.to_path_buf(), err))?;

        Ok(Self { output, path: path.to_path_buf(), stats })
    }
}

impl IteratorWriter<clang::Entry> for ClangOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), WriterError> {
        let stats = Arc::clone(&self.stats);

        // Count entries as they are written
        let counted_entries = entries.inspect(move |_| {
            stats.entries_written.fetch_add(1, Ordering::Relaxed);
        });

        JsonCompilationDatabase::write(self.output, counted_entries)
            .map_err(|err| WriterError::Io(self.path, err))
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

use super::IteratorWriter;
use super::file::write_entries;
use crate::output::SerializationError;
use crate::output::clang;
use crate::output::statistics::OutputStatistics;
use std::io;
use std::sync::Arc;

/// The type represents a writer that writes JSON compilation database
/// entries directly to standard output, for `bear semantic --output -`.
///
/// Unlike [`super::file::ClangOutputWriter`], there is no temp file or
/// rename step: a stream has no atomic-replace semantics and cannot be
/// appended to, so this writer is only ever the base of a pipeline that
/// skips the atomic and append decorators.
pub(crate) struct ClangStdoutOutputWriter {
    stats: Arc<OutputStatistics>,
}

impl ClangStdoutOutputWriter {
    pub(crate) fn new(stats: Arc<OutputStatistics>) -> Self {
        Self { stats }
    }
}

impl IteratorWriter<clang::Entry> for ClangStdoutOutputWriter {
    fn write(self, entries: impl Iterator<Item = clang::Entry>) -> Result<(), SerializationError> {
        let output = io::BufWriter::new(io::stdout());
        write_entries(output, entries, self.stats)
    }
}

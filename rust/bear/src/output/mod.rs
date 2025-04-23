// SPDX-License-Identifier: GPL-3.0-or-later

use super::semantic;
use anyhow::Result;

pub mod clang;
pub mod filter_duplicates;
pub mod formatter;

/// The output writer trait is responsible for writing output file.
pub(crate) trait OutputWriter {
    /// Running the writer means to consume the compiler calls
    /// and write the entries to the output file.
    fn run(&self, _: impl Iterator<Item = semantic::CompilerCall>) -> Result<()>;
}

// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use serde_json::Value;

// The order-independent matching core lives in the shared `bear-test-tools`
// crate so the integration suite and the `cdb-compare` tool share one
// implementation. Re-exported here to keep the existing test import path.
pub use bear_test_tools::CompilationEntryMatcher;

/// Compilation database wrapper with assertion helpers
#[derive(Debug)]
pub struct CompilationDatabase {
    pub(super) entries: Vec<Value>,
}

impl CompilationDatabase {
    /// Assert the number of entries
    pub fn assert_count(&self, expected: usize) -> Result<()> {
        let actual = self.entries.len();
        if actual != expected {
            anyhow::bail!("Expected {} compilation entries, but found {}", expected, actual);
        }
        Ok(())
    }

    /// Assert that the database contains an entry matching the criteria
    pub fn assert_contains(&self, matcher: &CompilationEntryMatcher) -> Result<()> {
        let found = self.entries.iter().any(|entry| matcher.matches(entry));
        if !found {
            anyhow::bail!(
                "Expected to find compilation entry matching: {:?}\nActual entries: {:#?}",
                matcher,
                self.entries
            );
        }
        Ok(())
    }

    /// Get all entries
    pub fn entries(&self) -> &[Value] {
        &self.entries
    }
}

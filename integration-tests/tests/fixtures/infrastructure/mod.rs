// SPDX-License-Identifier: GPL-3.0-or-later

//! Test infrastructure for Bear integration tests
//!
//! This module provides utilities for setting up test environments,
//! running bear commands, and validating outputs.
//!
//! # Failure diagnostics
//!
//! When a test panics, `TestEnvironment::Drop` automatically dumps the last
//! captured `BearOutput` (stdout, stderr, exit code) to the test binary's
//! stderr. How rich that dump is depends on `RUST_LOG`:
//!
//! - Local default (no `RUST_LOG`) → `run_bear` sets `RUST_LOG=info`, so
//!   warn/info/error log lines are captured (tests that assert on them
//!   work) but per-event `debug` traces from the preload library are
//!   filtered out, keeping ccache-cached compilation stderr clean.
//! - `RUST_LOG=debug cargo test` → propagated; bear logs verbosely and
//!   failure dumps include the full per-event interception trace.
//! - CI sets `RUST_LOG=debug` so failures on platforms that can't be
//!   reproduced locally carry full diagnostic context without a re-run.
//!
//! `BEAR_TEST_PRESERVE_FAILURES=1` additionally preserves the temp directory
//! at `/tmp/bear-test-<test_name>-<pid>` on panic.

mod bear_output;
mod compilation_database;
mod install_environment;
mod intercept_events;
mod test_environment;

#[cfg(test)]
mod tests;

pub use bear_output::BearOutput;
pub use compilation_database::{CompilationDatabase, CompilationEntryMatcher};
pub use install_environment::InstallEnvironment;
pub use intercept_events::{EventMatcher, InterceptEvents};
pub use test_environment::TestEnvironment;

use anyhow::Result;
use std::path::Path;

/// Helper macros for common test patterns
#[macro_export]
macro_rules! bear_test {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            let env = TestEnvironment::new(stringify!($name))?;
            $body(&env)
        }
    };
}

#[macro_export]
macro_rules! compilation_entry {
    (file: $file:expr, directory: $dir:expr, arguments: $args:expr) => {
        $crate::fixtures::infrastructure::CompilationEntryMatcher::new()
            .file($file)
            .directory($dir)
            .arguments($args)
    };
}

#[macro_export]
macro_rules! event_matcher {
    (executable_path: $path:expr) => {
        $crate::fixtures::infrastructure::EventMatcher::new().executable_path($path)
    };
    (executable_name: $name:expr) => {
        $crate::fixtures::infrastructure::EventMatcher::new().executable_name($name)
    };
    (executable_path: $path:expr, arguments: $args:expr) => {
        $crate::fixtures::infrastructure::EventMatcher::new().executable_path($path).arguments($args)
    };
}

// Re-export the macros at module level for easier importing
#[allow(unused_imports)]
pub use compilation_entry;
#[allow(unused_imports)]
pub use event_matcher;

/// Helper function to get the appropriate compiler command for build scripts
/// Always uses just the filename to ensure compatibility across all platforms
pub fn filename_of(compiler_path: &str) -> String {
    Path::new(compiler_path).file_name().unwrap().to_string_lossy().to_string()
}

// Test helper functions for common operations
#[allow(dead_code)]
pub fn touch_file(env: &TestEnvironment, path: &str) -> Result<()> {
    env.create_source_files(&[(path, "")])?;
    Ok(())
}

#[allow(dead_code)]
pub fn create_c_file(env: &TestEnvironment, path: &str, content: &str) -> Result<()> {
    env.create_source_files(&[(path, content)])?;
    Ok(())
}

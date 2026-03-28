// SPDX-License-Identifier: GPL-3.0-or-later

//! Test infrastructure for Bear integration tests
//!
//! This module provides utilities for setting up test environments,
//! running bear commands, and validating outputs. It's designed to
//! replicate the functionality of the Python/lit test suite.
//!
//! # Verbose Output Support
//!
//! The infrastructure supports verbose output display for debugging failed tests:
//!
//! ## Automatic Verbose Mode
//! Set `BEAR_TEST_VERBOSE=1` environment variable to automatically show bear output
//! when any test fails (panics).
//!
//! ## Manual Verbose Control
//! ```ignore
//! let env = TestEnvironment::new_with_verbose("test_name", true)?;
//! let output = env.run_bear(&["--output", "db.json", "--", "make"])?;
//!
//! // Show output only if verbose mode is enabled
//! output.show_verbose_if_enabled();
//!
//! // Force show output regardless of verbose setting
//! output.force_show_verbose();
//!
//! // Show last bear output from environment
//! env.show_last_bear_output();
//! ```
//!
//! ## Macro Support
//! ```ignore
//! bear_test!(my_test, verbose: true, |env| {
//!     let output = env.run_bear(&["semantic"])?;
//!     let db = env.load_compilation_database("compile_commands.json")?;
//!     db.assert_count(1)?; // Will show verbose output if this fails
//!     Ok(())
//! });
//! ```

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
    ($name:ident, verbose: $verbose:expr, $body:expr) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            let env = TestEnvironment::new_with_verbose(stringify!($name), $verbose)?;
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

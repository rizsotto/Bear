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
use serde_json::json;
use std::path::Path;

/// Source content shared by the single-driver / launcher / duplicate-collapse
/// semantic fixtures below: none of them care about the source's contents,
/// only that a file named `hello.c` exists next to the crafted events file.
const HELLO_C_SOURCE: &str = "int main() { return 0; }";

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

/// Shared core for the per-compiler-family "recognition-style" semantic
/// tests: create `dummy_executables` as empty files alongside a
/// `source_file` (with `source_content`), write one event whose
/// `executable`/`arguments` are `argv`, run `bear semantic`, and assert the
/// resulting database holds exactly one entry for `source_file` whose
/// recorded arguments equal `expected_args`.
fn assert_semantic_event_yields_entry_for_source(
    env_name: &str,
    dummy_executables: &[&str],
    argv: &[&str],
    expected_args: &[&str],
    source_file: &str,
    source_content: &str,
) -> Result<()> {
    let env = TestEnvironment::new(env_name)?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let event = json!({
        "executable": argv[0],
        "arguments": argv,
        "working_dir": temp_dir,
        "environment": {}
    });
    let events_content = event.to_string();

    let mut files: Vec<(&str, &str)> =
        vec![("events.json", events_content.as_str()), (source_file, source_content)];
    for name in dummy_executables {
        files.push((name, ""));
    }
    env.create_source_files(&files)?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: source_file.to_string(),
        directory: temp_dir.clone(),
        arguments: expected_args.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    ))?;

    Ok(())
}

/// Shared core for the per-compiler-family "recognition-style" semantic
/// tests whose source is `hello.c`: create `dummy_executables` as empty
/// files alongside a `hello.c` source, write one event whose
/// `executable`/`arguments` are `argv`, run `bear semantic`, and assert the
/// resulting database holds exactly one entry for `hello.c` whose recorded
/// arguments equal `expected_args`.
fn assert_semantic_event_yields_entry(
    env_name: &str,
    dummy_executables: &[&str],
    argv: &[&str],
    expected_args: &[&str],
) -> Result<()> {
    assert_semantic_event_yields_entry_for_source(
        env_name,
        dummy_executables,
        argv,
        expected_args,
        "hello.c",
        HELLO_C_SOURCE,
    )
}

/// Common shape used by the per-compiler-family recognition tests (mpi
/// wrapper, cray, hipcc, qnx, emcc, ti, xc8): a bare `<driver> -c
/// hello.c`-style invocation must be recognized and its argv recorded
/// verbatim as the sole compilation entry. Creates a dummy executable named
/// `argv[0]` on disk (recognition only inspects the basename, but the path
/// must exist).
#[allow(dead_code)]
pub fn assert_driver_yields_single_entry(env_name: &str, argv: &[&str]) -> Result<()> {
    assert_semantic_event_yields_entry(env_name, &[argv[0]], argv, argv)
}

/// Same shape as [`assert_driver_yields_single_entry`], but for a family
/// whose source isn't a `hello.c` translation unit (e.g. an assembler's
/// `hello.asm`): the entry is asserted against `source_file`/
/// `source_content` instead of the fixed `hello.c` default. Creates a dummy
/// executable named `argv[0]` on disk.
#[allow(dead_code)]
pub fn assert_driver_yields_single_entry_for_source(
    env_name: &str,
    argv: &[&str],
    source_file: &str,
    source_content: &str,
) -> Result<()> {
    assert_semantic_event_yields_entry_for_source(
        env_name,
        &[argv[0]],
        argv,
        argv,
        source_file,
        source_content,
    )
}

/// Launcher shape (e.g. icecc): the event's argv is the launcher token
/// followed by the real compiler invocation, but the recorded entry drops
/// the launcher token -- `expected_args` is what must survive.
/// `dummy_executables` must list every basename referenced in `argv` that
/// needs to exist on disk (the launcher and the wrapped compiler).
#[allow(dead_code)]
pub fn assert_launcher_execution_yields_entry(
    env_name: &str,
    dummy_executables: &[&str],
    argv: &[&str],
    expected_args: &[&str],
) -> Result<()> {
    assert_semantic_event_yields_entry(env_name, dummy_executables, argv, expected_args)
}

/// Duplicate-collapse shape (e.g. mpi wrapper + child compiler, emcc + clang
/// child): in preload mode a driver's own execution and its intercepted
/// child compiler process both produce an event for the same source. The
/// default duplicate filter (directory+file match) must collapse the pair
/// to one entry, keeping the FIRST event's argv (the driver's, not the
/// child's, since it comes first in the event stream).
#[allow(dead_code)]
pub fn assert_duplicate_events_collapse_to_first(
    env_name: &str,
    first_argv: &[&str],
    second_argv: &[&str],
) -> Result<()> {
    let env = TestEnvironment::new(env_name)?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let first_event = json!({
        "executable": first_argv[0],
        "arguments": first_argv,
        "working_dir": temp_dir,
        "environment": {}
    });
    let second_event = json!({
        "executable": second_argv[0],
        "arguments": second_argv,
        "working_dir": temp_dir,
        "environment": {}
    });
    let events_content = format!("{}\n{}", first_event, second_event);

    env.create_source_files(&[
        ("events.json", events_content.as_str()),
        ("hello.c", HELLO_C_SOURCE),
        (first_argv[0], ""),
        (second_argv[0], ""),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "hello.c".to_string(),
        directory: temp_dir.clone(),
        arguments: first_argv.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    ))?;

    Ok(())
}

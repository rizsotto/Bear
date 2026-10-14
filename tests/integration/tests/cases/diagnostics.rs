// SPDX-License-Identifier: GPL-3.0-or-later

//! User-facing error messages.
//!
//! Each case runs Bear without opting into logging and checks the one
//! diagnostic line it prints. The line is matched exactly up to the cause,
//! which is the operating system's text and varies by platform.

use crate::fixtures::infrastructure::TestEnvironment;
use anyhow::Result;

/// How a case prepares the test directory before Bear runs.
type Setup = fn(&TestEnvironment) -> Result<()>;

fn nothing(_: &TestEnvironment) -> Result<()> {
    Ok(())
}

fn output_is_a_directory(env: &TestEnvironment) -> Result<()> {
    std::fs::create_dir(env.test_dir().join("out"))?;
    Ok(())
}

fn invalid_existing_database(env: &TestEnvironment) -> Result<()> {
    env.create_source_files(&[("compile_commands.json", "this is not json")])
}

fn broken_configuration(env: &TestEnvironment) -> Result<()> {
    env.create_source_files(&[("broken.yml", "schema: \"4.2\"\nbogus: [\n")])
}

/// Runs Bear and returns its standard error lines, checking the exit status.
fn run(env: &TestEnvironment, args: &[&str], stdin: &[u8], succeeds: bool) -> Result<Vec<String>> {
    let result = env.run_bear_with_stdin_default_log(args, stdin)?;
    if succeeds {
        result.assert_success()?;
    } else {
        result.assert_failure()?;
    }
    Ok(result.stderr().lines().map(str::to_string).collect())
}

/// Asserts that `lines` is a single diagnostic that starts with `expected`
/// and carries a cause after it.
fn assert_one_diagnostic(case: &str, lines: &[String], expected: &str) {
    assert_eq!(lines.len(), 1, "{case}: expected one diagnostic line, got {lines:?}");
    let line = &lines[0];
    assert!(line.starts_with(expected), "{case}: expected {expected:?}..., got {line:?}");
    assert!(line.len() > expected.len(), "{case}: no cause after {expected:?} in {line:?}");
}

// Requirements: cli-diagnostic-format
#[test]
fn errors_name_the_file_and_the_cause() -> Result<()> {
    let cases: [(&str, Setup, &[&str], &str); 8] = [
        (
            "output directory is missing",
            nothing,
            &["--output", "missing/db.json", "--", "nosuchcmd"],
            "bear: error: cannot create missing/db.json: ",
        ),
        (
            "output is a directory",
            output_is_a_directory,
            &["--output", "out", "--", "nosuchcmd"],
            "bear: error: cannot create out: ",
        ),
        (
            "events output directory is missing",
            nothing,
            &["intercept", "--output", "missing/events.json", "--", "nosuchcmd"],
            "bear: error: cannot create missing/events.json: ",
        ),
        (
            "events input is missing",
            nothing,
            &["semantic", "--input", "missing.json", "--output", "db.json"],
            "bear: error: cannot read missing.json: ",
        ),
        (
            "shell text input is missing",
            nothing,
            &["parse-sh", "--input", "missing.sh", "--output", "db.json"],
            "bear: error: cannot read missing.sh: ",
        ),
        (
            "configuration is missing",
            nothing,
            &["--config", "missing.yml", "--", "nosuchcmd"],
            "bear: error: cannot read missing.yml: ",
        ),
        (
            "configuration cannot be parsed",
            broken_configuration,
            &["--config", "broken.yml", "--", "nosuchcmd"],
            "bear: error: cannot parse broken.yml: ",
        ),
        ("build cannot start", nothing, &["--", "nosuchcmd"], "bear: error: cannot run 'nosuchcmd': "),
    ];

    for (case, setup, args, expected) in cases {
        let env = TestEnvironment::new("diagnostics_errors")?;
        setup(&env)?;

        let sut = run(&env, args, b"", false)?;

        assert_one_diagnostic(case, &sut, expected);
    }

    Ok(())
}

// Requirements: cli-diagnostic-format
#[test]
#[cfg(unix)]
fn error_when_the_output_cannot_be_written() -> Result<()> {
    // A device that fails every write; not every unix has one.
    if !std::path::Path::new("/dev/full").exists() {
        return Ok(());
    }
    let env = TestEnvironment::new("diagnostics_write_error")?;

    let sut = run(&env, &["parse-sh", "--output", "/dev/full"], b"gcc -c a.c\n", false)?;

    assert_one_diagnostic("disk full", &sut, "bear: error: cannot write /dev/full: ");
    Ok(())
}

// Requirements: cli-diagnostic-format, output-append
#[test]
fn warning_when_previous_entries_cannot_be_read() -> Result<()> {
    let env = TestEnvironment::new("diagnostics_append_warning")?;
    invalid_existing_database(&env)?;

    let sut =
        run(&env, &["parse-sh", "--append", "--output", "compile_commands.json"], b"gcc -c a.c\n", true)?;

    assert_one_diagnostic(
        "invalid previous database",
        &sut,
        "bear: warning: cannot read previous entries from compile_commands.json: ",
    );
    env.load_compilation_database("compile_commands.json")?.assert_count(1)?;
    Ok(())
}

// Requirements: cli-diagnostic-format, output-append
#[test]
#[cfg(unix)]
fn error_when_previous_entries_cannot_be_opened() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let env = TestEnvironment::new("diagnostics_append_unreadable")?;
    invalid_existing_database(&env)?;
    let database = env.test_dir().join("compile_commands.json");
    std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o000))?;
    // Permissions do not stop a privileged user (root), so there is nothing to test.
    if std::fs::File::open(&database).is_ok() {
        return Ok(());
    }

    let sut = run(&env, &["--append", "--output", "compile_commands.json", "--", "nosuchcmd"], b"", false)?;

    assert_one_diagnostic(
        "unreadable previous database",
        &sut,
        "bear: error: cannot read compile_commands.json: ",
    );
    Ok(())
}

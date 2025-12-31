// SPDX-License-Identifier: GPL-3.0-or-later

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::TestEnvironment;
use anyhow::Result;
#[cfg(has_executable_sleep)]
use assert_cmd::cargo::cargo_bin;
#[cfg(has_executable_sleep)]
use std::process::{Command as StdCommand, Stdio};
#[cfg(has_executable_sleep)]
use std::time::Instant;

#[test]
fn exit_code_for_empty_arguments() -> Result<()> {
    // Executing Bear with no arguments should return a non-zero exit code,
    // and print usage information.
    let env = TestEnvironment::new("exit_code_for_empty_arguments")?;

    let result = env.run_bear(&[])?;
    result.assert_failure()?;
    assert!(result.stderr().contains("Usage: bear"));
    Ok(())
}

#[test]
fn exit_code_for_help() -> Result<()> {
    // Executing help and subcommand help should always has zero exit code,
    // and print out usage information
    let env = TestEnvironment::new("exit_code_for_help")?;

    // Test main help
    let result = env.run_bear(&["--help"])?;
    result.assert_success()?;
    assert!(result.stdout().contains("Usage: bear"));

    // Test intercept help
    let result = env.run_bear(&["intercept", "--help"])?;
    result.assert_success()?;
    assert!(result.stdout().contains("Usage: bear"));

    // Test semantic help
    let result = env.run_bear(&["semantic", "--help"])?;
    result.assert_success()?;
    assert!(result.stdout().contains("Usage: bear"));

    Ok(())
}

#[test]
fn exit_code_for_invalid_argument() -> Result<()> {
    // Executing Bear with an invalid argument should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    let env = TestEnvironment::new("exit_code_for_invalid_argument")?;

    let result = env.run_bear(&["invalid_argument"])?;
    result.assert_failure()?;
    assert!(result.stderr().contains("error: unexpected argument"));
    Ok(())
}

#[test]
#[cfg(target_os = "linux")] // FIXME: compiler wrappers does not work yet
fn exit_code_for_non_existing_command() -> Result<()> {
    // Executing a non-existing command should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    let env = TestEnvironment::new("exit_code_for_non_existing_command")?;

    let result = env.run_bear(&["--", "invalid_command"])?;
    result.assert_failure()?;
    assert!(result
        .stderr()
        .contains("Bear: Executor error: Failed to spawn child process"));
    Ok(())
}

#[test]
#[cfg(target_os = "linux")] // FIXME: compiler wrappers does not work yet
#[cfg(has_executable_true)]
fn exit_code_for_true() -> Result<()> {
    // When the executed command returns successfully, Bear exit code should be zero.
    let env = TestEnvironment::new("exit_code_for_true")?;

    let result = env.run_bear(&["--", TRUE_PATH])?;
    result.assert_success()?;
    Ok(())
}

#[test]
#[cfg(has_executable_false)]
fn exit_code_for_false() -> Result<()> {
    // When the executed command returns unsuccessfully, Bear exit code should be non-zero.
    let env = TestEnvironment::new("exit_code_for_false")?;

    let result = env.run_bear(&["--", FALSE_PATH])?;
    result.assert_failure()?;
    Ok(())
}

#[test]
#[cfg(has_executable_sleep)]
fn exit_code_when_signaled() -> Result<()> {
    // When the bear process is signaled, Bear exit code should be non-zero.
    // And should terminate the child process and return immediately.
    let env = TestEnvironment::new("exit_code_when_signaled")?;

    let mut cmd = StdCommand::new(cargo_bin(BEAR_BIN));
    cmd.current_dir(env.temp_dir())
        .arg("--")
        .arg(SLEEP_PATH)
        .arg("10")
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().expect("Failed to spawn command");

    // Wait 200ms to ensure that the sleep command was also executed
    std::thread::sleep(std::time::Duration::from_millis(200));

    let kill_time = Instant::now();
    child.kill().expect("Failed to signal the process");
    let status = child.wait().expect("Failed to wait for command");
    let wait_end = Instant::now();

    assert!(!status.success());
    assert!(
        wait_end.duration_since(kill_time).as_secs() < 1,
        "Process took too long to terminate.",
    );
    Ok(())
}

/// Test that bear returns 0 for successful compilation interception
#[cfg(has_executable_compiler_c)]
#[test]
fn exit_code_for_successful_compilation() -> Result<()> {
    let env = TestEnvironment::new("exit_code_for_successful_compilation")?;

    // Create a simple source file
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let result = env.run_bear(&[
        "--output",
        "compile_commands.json",
        "--",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;
    result.assert_success()?;

    // Verify compilation database was created
    assert!(env.file_exists("compile_commands.json"));
    Ok(())
}

/// Test that bear propagates build failure exit codes
#[cfg(has_executable_compiler_c)]
#[test]
fn exit_code_for_failed_compilation() -> Result<()> {
    let env = TestEnvironment::new("exit_code_for_failed_compilation")?;

    // Create an invalid source file that will cause compilation to fail
    env.create_source_files(&[("invalid.c", "this is not valid C code")])?;

    let result = env.run_bear(&[
        "--output",
        "compile_commands.json",
        "--",
        COMPILER_C_PATH,
        "-c",
        "invalid.c",
    ])?;
    result.assert_failure()?;
    Ok(())
}

/// Test that bear returns 0 when no build commands are executed
#[cfg(has_executable_true)]
#[test]
fn exit_code_for_empty_build() -> Result<()> {
    let env = TestEnvironment::new("exit_code_for_empty_build")?;

    let result = env.run_bear(&["--output", "compile_commands.json", "--", TRUE_PATH])?;
    result.assert_success()?;

    // Should create empty compilation database
    assert!(env.file_exists("compile_commands.json"));
    let content = env.read_file("compile_commands.json")?;
    assert_eq!(content.trim(), "[]");
    Ok(())
}

// Intercept mode exit code tests

/// Test that intercept command returns 0 for successful interception
#[cfg(has_executable_true)]
#[test]
fn intercept_exit_code_for_success() -> Result<()> {
    let env = TestEnvironment::new("intercept_exit_code_for_success")?;

    let result = env.run_bear(&["intercept", "--output", "events.json", "--", TRUE_PATH])?;
    result.assert_success()?;
    Ok(())
}

/// Test that intercept command propagates command failure exit codes
#[cfg(has_executable_false)]
#[test]
fn intercept_exit_code_for_failure() -> Result<()> {
    let env = TestEnvironment::new("intercept_exit_code_for_failure")?;

    let result = env.run_bear(&["intercept", "--output", "events.json", "--", FALSE_PATH])?;
    result.assert_failure()?;
    Ok(())
}

// Semantic mode exit code tests (note: this is now called 'semantic' not 'citnames')

/// Test that semantic command returns 0 for valid input
#[test]
fn semantic_exit_code_for_success() -> Result<()> {
    let env = TestEnvironment::new("semantic_exit_code_for_success")?;

    // Create a sample events file
    let events_content = r#"{"pid":12345,"execution":{"executable":"/usr/bin/gcc","arguments":["-c","test.c"],"working_dir":"/tmp","environment":{}}}"#;
    env.create_source_files(&[("events.json", events_content)])?;

    let result = env.run_bear(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;
    result.assert_success()?;
    Ok(())
}

/// Test that semantic command with missing input file returns non-zero
#[test]
fn semantic_exit_code_for_missing_input() -> Result<()> {
    let env = TestEnvironment::new("semantic_exit_code_for_missing_input")?;

    let result = env.run_bear(&[
        "semantic",
        "--input",
        "nonexistent.json",
        "--output",
        "compile_commands.json",
    ])?;
    result.assert_failure()?;
    Ok(())
}

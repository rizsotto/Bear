// SPDX-License-Identifier: GPL-3.0-or-later

use super::constants::*;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
#[cfg(has_executable_sleep)]
use std::process::{Command as StdCommand, Stdio};
#[cfg(has_executable_sleep)]
use std::time::Instant;

#[test]
fn test_exit_code_for_empty_arguments() {
    // Executing Bear with no arguments should return a non-zero exit code,
    // and print usage information.
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: bear"));
}

#[test]
fn test_exit_code_for_help() {
    // Executing help and subcommand help should always has zero exit code,
    // and print out usage information
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: bear"));

    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("intercept")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: bear"));

    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("semantic")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: bear"));
}

#[test]
fn test_exit_code_for_invalid_argument() {
    // Executing Bear with an invalid argument should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("invalid_argument")
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: unexpected argument"));
}

#[test]
#[cfg(target_os = "linux")] // FIXME: compiler wrappers does not work yet
fn test_exit_code_for_non_existing_command() {
    // Executing a non-existing command should always has non-zero exit code,
    // and print relevant information about the reason about the failure.
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .args(["--", "invalid_command"])
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to execute the build command",
        ));
}

#[test]
#[cfg(target_os = "linux")] // FIXME: compiler wrappers does not work yet
#[cfg(has_executable_true)]
fn test_exit_code_for_true() {
    // When the executed command returns successfully, Bear exit code should be zero.
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("--")
        .arg(TRUE_PATH)
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .success();
}

#[test]
#[cfg(has_executable_false)]
fn test_exit_code_for_false() {
    // When the executed command returns unsuccessfully, Bear exit code should be non-zero.
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("--")
        .arg(FALSE_PATH)
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .assert()
        .failure();
}

#[test]
#[cfg(has_executable_sleep)]
fn test_exit_code_when_signaled() {
    // When the bear process is signaled, Bear exit code should be non-zero.
    // And should terminate the child process and return immediately.

    let mut cmd = StdCommand::new(cargo_bin(BEAR_BIN));
    cmd.arg("--")
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
}

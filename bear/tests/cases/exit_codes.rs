// SPDX-License-Identifier: GPL-3.0-or-later

use super::constants::{BEAR_BIN, FALSE_PATH, SLEEP_PATH, TRUE_PATH};

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use predicates::prelude::*;
#[cfg(has_executable_sleep)]
use std::process::{Command as StdCommand, Stdio};
#[cfg(has_executable_sleep)]
use std::time::Instant;

#[test]
fn test_exit_code_for_empty_arguments() {
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage: bear"));
}

#[test]
fn test_exit_code_for_help() {
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
fn test_exit_code_for_invalid_command() {
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("invalid_command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error: unexpected argument"));
}

#[test]
#[cfg(has_executable_true)]
fn test_exit_code_for_true() {
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("--")
        .arg(TRUE_PATH)
        .assert()
        .success();
}

#[test]
#[cfg(has_executable_false)]
fn test_exit_code_for_false() {
    Command::cargo_bin(BEAR_BIN)
        .unwrap()
        .arg("--")
        .arg(FALSE_PATH)
        .assert()
        .failure();
}

#[test]
#[cfg(has_executable_sleep)]
fn test_exit_code_when_signaled() {
    // Prepare the command
    let mut cmd = StdCommand::new(cargo_bin(BEAR_BIN));
    cmd.arg("--")
        .arg(SLEEP_PATH)
        .arg("10")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    // Start the command
    let mut child = cmd.spawn().expect("Failed to spawn command");

    // Wait 200ms to ensure that the sleep command was also executed
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Send a termination signal to the process and record the time
    let kill_time = Instant::now();
    child.kill().expect("Failed to signal the process");

    // Wait for the process to complete and record the time
    let status = child.wait().expect("Failed to wait for command");
    let wait_end = Instant::now();

    // Assert that the exit status is not zero
    assert!(!status.success());

    // Assert that the process stopped right after the kill call (less than 1 second)
    let time_diff = wait_end.duration_since(kill_time);
    assert!(
        time_diff.as_secs() < 1,
        "Process took too long to terminate: {:?}",
        time_diff
    );
}

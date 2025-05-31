// SPDX-License-Identifier: GPL-3.0-or-later

use super::constants::{BEAR_BIN, FALSE_PATH, TRUE_PATH};

use assert_cmd::Command;
use predicates::prelude::*;

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

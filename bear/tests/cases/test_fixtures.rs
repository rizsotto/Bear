// SPDX-License-Identifier: GPL-3.0-or-later

use super::constants::*;

use assert_cmd::Command;

#[cfg(has_executable_true)]
#[test]
fn test_true_help() {
    Command::new(TRUE_PATH).assert().success();
}

#[cfg(has_executable_false)]
#[test]
fn test_false_help() {
    Command::new(FALSE_PATH).assert().failure();
}

#[cfg(has_executable_echo)]
#[test]
fn test_echo_help() {
    Command::new(ECHO_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_sleep)]
#[test]
fn test_sleep_help() {
    Command::new(SLEEP_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_shell)]
#[test]
fn test_shell_help() {
    Command::new(SHELL_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_make)]
#[test]
fn test_make_help() {
    Command::new(MAKE_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_compiler_c)]
#[test]
fn test_compiler_c_help() {
    Command::new(COMPILER_C_PATH)
        .arg("--help")
        .assert()
        .success();
}

#[cfg(has_executable_compiler_cxx)]
#[test]
fn test_compiler_cxx_help() {
    Command::new(COMPILER_CXX_PATH)
        .arg("--help")
        .assert()
        .success();
}

#[cfg(has_executable_compiler_fortran)]
#[test]
fn test_compiler_fortran_help() {
    Command::new(COMPILER_FORTRAN_PATH)
        .arg("--help")
        .assert()
        .success();
}

#[cfg(has_executable_compiler_cuda)]
#[test]
fn test_compiler_cuda_help() {
    Command::new(COMPILER_CUDA_PATH)
        .arg("--help")
        .assert()
        .success();
}

#[cfg(has_executable_libtool)]
#[test]
fn test_libtool_help() {
    Command::new(LIBTOOL_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_fakeroot)]
#[test]
fn test_fakeroot_help() {
    Command::new(FAKEROOT_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_valgrind)]
#[test]
fn test_valgrind_help() {
    Command::new(VALGRIND_PATH).arg("--help").assert().success();
}

#[cfg(has_executable_ar)]
#[test]
fn test_ar_help() {
    Command::new(AR_PATH).arg("--help").assert().success();
}

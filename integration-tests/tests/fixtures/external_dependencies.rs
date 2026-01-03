// SPDX-License-Identifier: GPL-3.0-or-later

use super::constants::*;

use assert_cmd::Command;

#[cfg(has_executable_true)]
#[test]
fn true_works() {
    Command::new(TRUE_PATH).assert().success();
}

#[cfg(has_executable_false)]
#[test]
fn false_works() {
    Command::new(FALSE_PATH).assert().failure();
}

#[cfg(has_executable_echo)]
#[test]
fn echo_works() {
    // Testing echo with as executing to print out a value.
    // Testing with `--help` or `--version` is not a portable test.
    Command::new(ECHO_PATH).arg("testing").assert().success();
}

#[cfg(has_executable_sleep)]
#[test]
fn sleep_works() {
    // Testing sleep with a zero second value.
    // Testing with `--help` or `--version` is not a portable test.
    Command::new(SLEEP_PATH).arg("0").assert().success();
}

#[cfg(has_executable_shell)]
#[test]
fn shell_works() {
    // Testing shell to execute a built it function.
    // Testing with `--help` or `--version` is not a portable test. Debian `dash` is failing with those arguments.
    Command::new(SHELL_PATH).args(["-c", "true"]).assert().success();
}

#[cfg(has_executable_make)]
#[test]
fn make_works() {
    // Testing make by querying its version.
    Command::new(MAKE_PATH).arg("--version").assert().success();
}

#[cfg(has_executable_compiler_c)]
#[test]
fn compiler_c_works() {
    // Testing compiler by querying its version.
    Command::new(COMPILER_C_PATH).arg("--version").assert().success();
}

#[cfg(has_executable_compiler_cxx)]
#[test]
fn compiler_cxx_works() {
    // Testing compiler by querying its version.
    Command::new(COMPILER_CXX_PATH).arg("--version").assert().success();
}

#[cfg(has_executable_compiler_fortran)]
#[test]
fn compiler_fortran_works() {
    // Testing compiler by querying its version.
    Command::new(COMPILER_FORTRAN_PATH).arg("--version").assert().success();
}

#[cfg(has_executable_compiler_cuda)]
#[test]
fn compiler_cuda_works() {
    // Testing compiler by querying its version.
    Command::new(COMPILER_CUDA_PATH).arg("--version").assert().success();
}

#[cfg(not(target_os = "macos"))]
#[cfg(has_executable_libtool)]
#[test]
fn libtool_works() {
    // Testing libtool by querying its version.
    // FIXME: libtool does not have version or help parameters on macOS
    Command::new(LIBTOOL_PATH).arg("--version").assert().success();
}

#[cfg(has_executable_fakeroot)]
#[test]
fn fakeroot_works() {
    // Testing fakeroot by querying its version.
    Command::new(FAKEROOT_PATH).arg("--version").assert().success();
}

#[cfg(has_executable_valgrind)]
#[test]
fn valgrind_works() {
    // Testing valgrind by querying its version.
    Command::new(VALGRIND_PATH).arg("--version").assert().success();
}

#[cfg(not(target_os = "macos"))]
#[cfg(has_executable_ar)]
#[test]
fn ar_works() {
    // Testing ar by querying its version.
    // FIXME: ar does not have version or help parameters on macOS
    Command::new(AR_PATH).arg("--version").assert().success();
}

// SPDX-License-Identifier: GPL-3.0-or-later

pub const BEAR_BIN: &str = "bear";

#[cfg(has_executable_true)]
pub const TRUE_PATH: &str = env!("TRUE_PATH");
#[cfg(has_executable_false)]
pub const FALSE_PATH: &str = env!("FALSE_PATH");
#[cfg(has_executable_echo)]
pub const ECHO_PATH: &str = env!("ECHO_PATH");
#[cfg(has_executable_sleep)]
pub const SLEEP_PATH: &str = env!("SLEEP_PATH");
#[cfg(has_executable_shell)]
pub const SHELL_PATH: &str = env!("SHELL_PATH");
#[cfg(has_executable_make)]
pub const MAKE_PATH: &str = env!("MAKE_PATH");
#[cfg(has_executable_compiler_c)]
pub const COMPILER_C_PATH: &str = env!("COMPILER_C_PATH");
#[cfg(has_executable_compiler_cxx)]
pub const COMPILER_CXX_PATH: &str = env!("COMPILER_CXX_PATH");
#[cfg(has_executable_compiler_fortran)]
pub const COMPILER_FORTRAN_PATH: &str = env!("COMPILER_FORTRAN_PATH");
#[cfg(has_executable_compiler_cuda)]
pub const COMPILER_CUDA_PATH: &str = env!("COMPILER_CUDA_PATH");
#[cfg(has_executable_libtool)]
pub const LIBTOOL_PATH: &str = env!("LIBTOOL_PATH");
#[cfg(has_executable_fakeroot)]
pub const FAKEROOT_PATH: &str = env!("FAKEROOT_PATH");
#[cfg(has_executable_valgrind)]
pub const VALGRIND_PATH: &str = env!("VALGRIND_PATH");
#[cfg(has_executable_ar)]
pub const AR_PATH: &str = env!("AR_PATH");

// Intercept artifact paths - only available when integration tests are enabled
#[cfg(feature = "allow-integration-tests")]
#[allow(dead_code)]
pub const WRAPPER_EXECUTABLE_PATH: &str = env!("WRAPPER_EXECUTABLE_PATH");
#[cfg(feature = "allow-integration-tests")]
#[allow(dead_code)]
pub const PRELOAD_LIBRARY_PATH: &str = env!("PRELOAD_LIBRARY_PATH");

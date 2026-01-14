// SPDX-License-Identifier: GPL-3.0-or-later

// Bear executable path - only available when integration tests are enabled
#[cfg(feature = "allow-integration-tests")]
#[allow(dead_code)]
pub const BEAR_EXECUTABLE_PATH: &str = env!("BEAR_EXECUTABLE_PATH");

#[cfg(has_executable_true)]
#[allow(dead_code)]
pub const TRUE_PATH: &str = env!("TRUE_PATH");
#[cfg(has_executable_false)]
#[allow(dead_code)]
pub const FALSE_PATH: &str = env!("FALSE_PATH");
#[cfg(has_executable_echo)]
#[allow(dead_code)]
pub const ECHO_PATH: &str = env!("ECHO_PATH");
#[cfg(has_executable_sleep)]
#[allow(dead_code)]
pub const SLEEP_PATH: &str = env!("SLEEP_PATH");
#[cfg(has_executable_shell)]
#[allow(dead_code)]
pub const SHELL_PATH: &str = env!("SHELL_PATH");
#[cfg(has_executable_make)]
#[allow(dead_code)]
pub const MAKE_PATH: &str = env!("MAKE_PATH");
#[cfg(has_executable_compiler_c)]
#[allow(dead_code)]
pub const COMPILER_C_PATH: &str = env!("COMPILER_C_PATH");
#[cfg(has_executable_compiler_cxx)]
#[allow(dead_code)]
pub const COMPILER_CXX_PATH: &str = env!("COMPILER_CXX_PATH");
#[cfg(has_executable_compiler_fortran)]
#[allow(dead_code)]
pub const COMPILER_FORTRAN_PATH: &str = env!("COMPILER_FORTRAN_PATH");
#[cfg(has_executable_compiler_cuda)]
#[allow(dead_code)]
pub const COMPILER_CUDA_PATH: &str = env!("COMPILER_CUDA_PATH");
#[cfg(has_executable_libtool)]
#[allow(dead_code)]
pub const LIBTOOL_PATH: &str = env!("LIBTOOL_PATH");
#[cfg(has_executable_fakeroot)]
#[allow(dead_code)]
pub const FAKEROOT_PATH: &str = env!("FAKEROOT_PATH");
#[cfg(has_executable_valgrind)]
#[allow(dead_code)]
pub const VALGRIND_PATH: &str = env!("VALGRIND_PATH");
#[cfg(has_executable_ar)]
#[allow(dead_code)]
pub const AR_PATH: &str = env!("AR_PATH");
#[cfg(has_executable_env)]
#[allow(dead_code)]
pub const ENV_PATH: &str = env!("ENV_PATH");
#[cfg(has_executable_cat)]
#[allow(dead_code)]
pub const CAT_PATH: &str = env!("CAT_PATH");
#[cfg(has_executable_ls)]
#[allow(dead_code)]
pub const LS_PATH: &str = env!("LS_PATH");
#[cfg(has_executable_mkdir)]
#[allow(dead_code)]
pub const MKDIR_PATH: &str = env!("MKDIR_PATH");
#[cfg(has_executable_rm)]
#[allow(dead_code)]
pub const RM_PATH: &str = env!("RM_PATH");

// Intercept artifact paths - only available when integration tests are enabled
#[cfg(feature = "allow-integration-tests")]
#[allow(dead_code)]
pub const WRAPPER_EXECUTABLE_PATH: &str = env!("WRAPPER_EXECUTABLE_PATH");
#[cfg(feature = "allow-integration-tests")]
#[allow(dead_code)]
pub const PRELOAD_LIBRARY_PATH: &str = env!("PRELOAD_LIBRARY_PATH");

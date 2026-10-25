// SPDX-License-Identifier: GPL-3.0-or-later

// Bear executable path - only available when integration tests are enabled
pub const DRIVER_EXECUTABLE_PATH: &str = env!("DRIVER_EXECUTABLE_PATH");
pub const WRAPPER_EXECUTABLE_PATH: &str = env!("WRAPPER_EXECUTABLE_PATH");
pub const PRELOAD_LIBRARY_PATH: &str = env!("PRELOAD_LIBRARY_PATH");

pub const DRIVER_EXECUTABLE: &str = env!("DRIVER_EXECUTABLE");
pub const WRAPPER_EXECUTABLE: &str = env!("WRAPPER_EXECUTABLE");
pub const PRELOAD_LIBRARY: &str = env!("PRELOAD_LIBRARY");

#[cfg(has_executable_true)]
pub const TRUE_PATH: &str = env!("TRUE_PATH");
#[cfg(has_executable_false)]
pub const FALSE_PATH: &str = env!("FALSE_PATH");
#[cfg(has_executable_echo)]
pub const ECHO_PATH: &str = env!("ECHO_PATH");
#[cfg(has_executable_sleep)]
pub const SLEEP_PATH: &str = env!("SLEEP_PATH");
#[cfg(has_executable_setsid)]
pub const SETSID_PATH: &str = env!("SETSID_PATH");
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
#[cfg(has_executable_env)]
pub const ENV_PATH: &str = env!("ENV_PATH");
#[cfg(has_executable_cat)]
pub const CAT_PATH: &str = env!("CAT_PATH");
#[cfg(has_executable_ls)]
pub const LS_PATH: &str = env!("LS_PATH");
#[cfg(has_executable_mkdir)]
pub const MKDIR_PATH: &str = env!("MKDIR_PATH");
#[cfg(has_executable_rm)]
pub const RM_PATH: &str = env!("RM_PATH");

pub const INSTALL_SCRIPT_PATH: &str = env!("INSTALL_SCRIPT_PATH");

pub const INTERCEPT_LIBDIR: &str = env!("INTERCEPT_LIBDIR");

pub const REPO_ROOT: &str = env!("REPO_ROOT");

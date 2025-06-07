//! Library for Bear interception via LD_PRELOAD
//!
//! This library provides system capability checks at build time.
//! The following cfg flags are available for conditional compilation:
//!
//! Headers: has_header_dlfcn_h, has_header_errno_h, has_header_unistd_h,
//!          has_header_spawn_h, has_header_stdio_h, has_header_stdlib_h
//!
//! Symbols: has_symbol_dlopen, has_symbol_dlsym, has_symbol_dlerror,
//!          has_symbol_dlclose, has_symbol_RTLD_NEXT, has_symbol_EACCES,
//!          has_symbol_ENOENT, has_symbol_execve, has_symbol_execv, etc.
//!
//! Example usage:
//! ```rust
//! #[cfg(has_symbol_dlopen)]
//! fn use_dlopen() { /* implementation */ }
//!
//! #[cfg(not(has_symbol_execveat))]
//! fn fallback_exec() { /* fallback implementation */ }
//! ```

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Version information for the library
#[no_mangle]
pub static LIBEAR_VERSION: &[u8; 6] = b"4.0.0\0";

/// Intercepted open function
///
/// # Safety
///
/// This is an FFI function intended for LD_PRELOAD interception.
#[no_mangle]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: libc::mode_t) -> c_int {
    // Log the file being opened if logging is initialized
    if !path.is_null() {
        if let Ok(path_str) = CStr::from_ptr(path).to_str() {
            log::debug!("libear: open called for path: {}", path_str);

            // Here we could implement additional logic for interception
            // For example, recording file access for compilation database
        }
    }

    // Call the real open function
    libc::open(path, flags, mode)
}

/// Intercepted execve function
///
/// # Safety
///
/// This is an FFI function intended for LD_PRELOAD interception.
#[no_mangle]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    // Log the process being executed
    if !path.is_null() {
        if let Ok(path_str) = CStr::from_ptr(path).to_str() {
            log::info!("libear: intercepted execution of: {}", path_str);

            // Here we could implement command interception logic
            // For example, recording compiler invocations
        }
    }

    // Call the real execve function
    libc::execve(path, argv, envp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = unsafe { CStr::from_ptr(LIBEAR_VERSION.as_ptr() as *const c_char) };
        assert_eq!(version.to_str().unwrap(), "4.0.0");
    }
}

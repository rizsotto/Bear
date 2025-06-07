// SPDX-License-Identifier: GPL-3.0-or-later

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

// Only include Linux implementation when building for Linux
#[cfg(target_os = "linux")]
mod implementation;

// Re-export Linux implementations when on Linux
#[cfg(target_os = "linux")]
pub use implementation::*;

/// Version information for the library
#[no_mangle]
pub static LIBEAR_VERSION: &[u8; 6] = b"4.0.0\0";

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[test]
    fn test_version() {
        let version = unsafe { CStr::from_ptr(LIBEAR_VERSION.as_ptr() as *const c_char) };
        assert_eq!(version.to_str().unwrap(), "4.0.0");
    }
}

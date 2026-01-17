// SPDX-License-Identifier: GPL-3.0-or-later

//! Library for Bear interception via LD_PRELOAD
//!
//! The library captures system calls and reports them to the collector.

// Only include Unix implementation when building for Unix
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
mod implementation;

// Re-export Unix implementations when on Unix
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
pub use implementation::*;

/// Version information for the library
#[unsafe(no_mangle)]
pub static LIBEAR_VERSION: &[u8; 6] = b"4.0.0\0";

// SPDX-License-Identifier: GPL-3.0-or-later

//! Library for Bear interception via LD_PRELOAD
//!
//! The library captures system calls and reports them to the collector.

// Only include Linux implementation when building for Linux
#[cfg(target_os = "linux")]
mod implementation;

// Re-export Linux implementations when on Linux
#[cfg(target_os = "linux")]
pub use implementation::*;

/// Version information for the library
#[no_mangle]
pub static LIBEAR_VERSION: &[u8; 6] = b"4.0.0\0";

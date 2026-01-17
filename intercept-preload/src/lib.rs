// SPDX-License-Identifier: GPL-3.0-or-later

//! Library for Bear interception via LD_PRELOAD
//!
//! The library captures system calls and reports them to the collector.

// Only include implementation when building on unix
#[cfg(target_family = "unix")]
mod implementation;

// Re-export implementations
#[cfg(target_family = "unix")]
pub use implementation::*;

/// Version information for the library
#[unsafe(no_mangle)]
pub static LIBEAR_VERSION: &[u8; 6] = b"4.0.0\0";

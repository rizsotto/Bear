// SPDX-License-Identifier: GPL-3.0-or-later

//! Library for Bear interception via LD_PRELOAD
//!
//! The library captures system calls and reports them to the collector.

// Only include implementation when building on unix
#[cfg(target_family = "unix")]
mod implementation;

#[cfg(target_family = "unix")]
mod session;

// Re-export implementations
#[cfg(target_family = "unix")]
pub use implementation::*;

/// Package version from Cargo.toml, used to derive the null-terminated C export below.
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version information for the library, exported as a null-terminated C string.
///
/// Derived automatically from the workspace version in `Cargo.toml` so it can
/// never drift out of sync.
#[unsafe(no_mangle)]
pub static LIBEXEC_VERSION: [u8; PKG_VERSION.len() + 1] = {
    let src = PKG_VERSION.as_bytes();
    let mut buf = [0u8; PKG_VERSION.len() + 1];
    let mut i = 0;
    while i < src.len() {
        buf[i] = src[i];
        i += 1;
    }
    // buf[src.len()] is already 0 from zero-initialization — the null terminator.
    buf
};

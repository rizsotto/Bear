// SPDX-License-Identifier: GPL-3.0-or-later

//! Integration test cases for Bear
//!
//! This module contains the actual integration tests that verify Bear's functionality
//! across different scenarios and use cases.
//!
//! ## Platform-Specific Test Behavior
//!
//! Bear's integration tests are designed to work across different platforms, but some
//! tests require specific interception mechanisms:
//!
//! ### Preload-based Tests (Linux, FreeBSD, NetBSD, OpenBSD, DragonFly)
//! - `intercept_posix`: All POSIX system call interception tests (execve, popen, etc.)
//! - `config`: Configuration tests that use preload-specific settings
//! - Some tests in `intercept`: Tests that require direct system call hooking
//!
//! ### Wrapper-based Tests (macOS, iOS, Windows, and all other platforms)
//! - `wrapper_based_interception`: Tests designed for wrapper-based interception
//! - General intercept tests that work with process monitoring
//!
//! ### Universal Tests (All platforms)
//! - `compilation_output`: End-to-end compilation database generation
//! - `exit_codes`: Command exit code handling
//! - Platform-agnostic functionality tests
//!
//! Tests use conditional compilation attributes (`#[cfg(target_os = "...")]`) to ensure
//! they only run on platforms where the required interception mechanism is available.

pub mod compilation_output;
pub mod config;
pub mod exit_codes;
pub mod intercept;
pub mod intercept_posix;
pub mod semantic;

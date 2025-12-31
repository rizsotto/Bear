// SPDX-License-Identifier: GPL-3.0-or-later

//! Test fixtures and infrastructure for Bear integration tests
//!
//! This module contains all the shared infrastructure, constants, and fixture tests
//! that are used across the Bear integration test suite.

pub mod constants;
pub mod external_dependencies;
pub mod infrastructure;

// Re-export commonly used items for convenience
// These are marked as allow unused since some modules may not use all items
#[allow(unused_imports)]
pub use constants::*;
#[allow(unused_imports)]
pub use infrastructure::*;

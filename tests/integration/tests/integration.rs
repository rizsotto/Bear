// SPDX-License-Identifier: GPL-3.0-or-later

//! Bear integration test suite
//!
//! This module organizes the Bear integration tests into two main components:
//! - `fixtures`: Test infrastructure, constants, and fixture tests for external dependencies
//! - `cases`: Actual integration test cases that verify Bear functionality

mod cases;
#[allow(
    dead_code,
    unused_imports,
    reason = "tests are gated on host tools, so helpers used only by skipped tests go unused"
)]
mod fixtures;

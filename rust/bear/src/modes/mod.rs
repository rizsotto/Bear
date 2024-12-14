// SPDX-License-Identifier: GPL-3.0-or-later

pub mod intercept;
pub mod semantic;
pub mod combined;

use std::process::ExitCode;

/// The mode trait is used to run the application in different modes.
pub trait Mode {
    fn run(self) -> anyhow::Result<ExitCode>;
}

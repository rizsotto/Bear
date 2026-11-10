// SPDX-License-Identifier: GPL-3.0-or-later

//! The driver-side (supervisor) half of the interception runtime.
//!
//! This crate owns the consumer side of the mechanism: it supervises the build
//! process, collects the executions reported over TCP, and sets up the build
//! environment that injects the interception (wrapper directory or preload
//! library). It depends on `intercept` for the shared `Execution` type and the
//! wire serialization, but never on `bear`, `config`, or `clap`, so the preload
//! cdylib - which depends on `intercept` only - never compiles supervisor-only
//! dependencies (`signal-hook`, `which`).

pub mod collector;
pub mod context;
pub mod installation;
pub mod runner;
pub mod supervise;
pub mod wrapper;

pub use collector::CollectorOnTcp;
pub use context::{Context, ContextError};
pub use installation::{InstallationLayout, LayoutError};
pub use runner::{BuildEnvironment, ConfigurationError};
pub use supervise::{GroupPolicy, Inherit, Leader, SuperviseError, supervise, supervise_execution};
pub use wrapper::{
    CONFIG_FILENAME, ConfigError, WrapperConfig, WrapperConfigReader, WrapperDirectory,
    WrapperDirectoryBuilder, WrapperDirectoryError,
};

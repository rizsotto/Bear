// SPDX-License-Identifier: GPL-3.0-or-later

//! Reporter module for command interception layer.
//!
//! This module provides abstractions and implementations for reporting intercepted events
//! to a remote collector. It defines error types for initialization and reporting failures,
//! a trait for reporting events, and a factory for creating reporter instances.
//!
//! The main responsibilities include:
//! - Defining the `Reporter` trait for sending events.
//! - Providing error types for initialization and reporting.
//! - Implementing a factory to create socket-based reporters.

use crate::Execution;
use crate::wire::{Address, SocketReporter};
use thiserror::Error;

/// Trait for reporting intercepted executions to a remote collector.
pub trait Reporter {
    /// Sends an execution to the remote collector.
    ///
    /// The execution is serialized and sent to the remote collector.
    /// The connection is opened and closed for each execution.
    fn report(&self, execution: Execution) -> Result<(), ReporterError>;
}

/// Errors that can occur while reporting events.
#[derive(Error, Debug)]
pub enum ReporterError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

/// Factory for creating reporter instances.
pub struct ReporterFactory;

impl ReporterFactory {
    /// Creates a new socket-based reporter using the destination from the environment.
    ///
    /// The created reporter is not connected yet; it only stores the destination address.
    pub fn create(address: Address) -> SocketReporter {
        SocketReporter::new(address)
    }
}

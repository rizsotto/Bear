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
//! - Implementing a factory to create TCP-based reporters.

use crate::intercept::{Event, tcp};
use std::net::SocketAddr;
use thiserror::Error;

/// Trait for reporting intercepted events to a remote collector.
pub trait Reporter {
    /// Sends an event to the remote collector.
    ///
    /// The event is wrapped in an envelope and sent to the remote collector.
    /// The TCP connection is opened and closed for each event.
    fn report(&self, event: Event) -> Result<(), ReporterError>;
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
    /// Creates a new TCP-based reporter using the destination from the environment.
    ///
    /// The created reporter is not connected yet; it only stores the destination address.
    pub fn create(address: SocketAddr) -> tcp::ReporterOnTcp {
        tcp::ReporterOnTcp::new(address)
    }
}

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
//!
//! # Usage
//!
//! Use `ReporterFactory::create()` to instantiate a reporter with a socket address, or
//! `ReporterFactory::create_as_ptr()` to obtain a raw pointer suitable for static/global usage.
//! The reporter sends events to a remote collector at the specified address.

use crate::intercept::{Event, tcp};
use std::net::SocketAddr;
use std::sync::atomic::AtomicPtr;
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
    pub fn create(address: SocketAddr) -> impl Reporter {
        tcp::ReporterOnTcp::new(address)
    }

    /// Creates a new reporter and returns it as an atomic pointer.
    ///
    /// This is useful for static/global usage where a stable pointer is required
    /// for the program's lifetime.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring the returned pointer is not used after
    /// the program terminates. The memory will be leaked intentionally to provide a
    /// stable pointer for the lifetime of the program.
    ///
    /// Returns a null pointer if reporter creation fails. Caller must check for null
    /// before dereferencing.
    pub fn create_as_ptr(address_str: &str) -> AtomicPtr<tcp::ReporterOnTcp> {
        match address_str.parse::<SocketAddr>() {
            Ok(address) => {
                // Leak the reporter to get a stable pointer for the lifetime of the program
                let boxed_reporter = Box::new(tcp::ReporterOnTcp::new(address));
                let ptr = Box::into_raw(boxed_reporter);

                AtomicPtr::new(ptr)
            }
            Err(err) => {
                log::warn!("Failed to create reporter: {err}");
                AtomicPtr::new(std::ptr::null_mut())
            }
        }
    }
}

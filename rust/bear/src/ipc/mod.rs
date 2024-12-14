// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the intercept reporting and collecting functionality.
//!
//! When a command execution is intercepted, the interceptor sends the event to the collector.
//! This happens in two different processes, requiring a communication channel between these
//! processes.
//!
//! The module provides abstractions for the reporter and the collector. And it also defines
//! the data structures that are used to represent the events.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

pub mod tcp;

/// Represents the remote sink of supervised process events.
///
/// This allows the reporters to send events to a remote collector.
pub trait Reporter {
    fn report(&self, event: Event) -> Result<(), anyhow::Error>;
}

/// Represents the local sink of supervised process events.
///
/// The collector is responsible for collecting the events from the reporters.
///
/// To share the collector between threads, we use the `Arc` type to wrap the
/// collector. This way we can clone the collector and send it to other threads.
pub trait Collector {
    /// Returns the address of the collector.
    ///
    /// The address is in the format of `ip:port`.
    fn address(&self) -> String;

    /// Collects the events from the reporters.
    ///
    /// The events are sent to the given destination channel.
    ///
    /// The function returns when the collector is stopped. The collector is stopped
    /// when the `stop` method invoked (from another thread).
    fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error>;

    /// Stops the collector.
    fn stop(&self) -> Result<(), anyhow::Error>;
}

/// Envelope is a wrapper around the event.
///
/// It contains the reporter id, the timestamp of the event and the event itself.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Envelope {
    pub rid: ReporterId,
    pub timestamp: u64,
    pub event: Event,
}

/// Represent a relevant life cycle event of a process.
///
/// In the current implementation, we only have one event, the `Started` event.
/// This event is sent when a process is started. It contains the process id
/// and the execution information.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event {
    pub pid: ProcessId,
    pub execution: Execution,
}

/// Execution is a representation of a process execution.
///
/// It does not contain information about the outcome of the execution,
/// like the exit code or the duration of the execution. It only contains
/// the information that is necessary to reproduce the execution.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Execution {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
}

impl fmt::Display for Execution {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Execution path={}, args=[{}]",
            self.executable.display(),
            self.arguments.join(",")
        )
    }
}

/// Reporter id is a unique identifier for a reporter.
///
/// It is used to identify the process that sends the execution report.
/// Because the OS PID is not unique across a single build (PIDs are
/// recycled), we need to use a new unique identifier to identify the process.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ReporterId(pub u64);

/// Process id is a OS identifier for a process.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ProcessId(pub u32);

// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the driver-side TCP collector.
//!
//! The collector is the receiver half of the interception wire protocol. It
//! listens on a loopback TCP port, accepts one connection per reported
//! execution, and reads each record using the shared
//! [`intercept::tcp::ExecutionWireSerializer`] wire format that the reporter
//! (`intercept::tcp::ReporterOnTcp`) writes.

use intercept::Execution;
use intercept::reporter::ReporterError;
use intercept::tcp::ExecutionWireSerializer;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Represents a TCP execution collector.
pub struct CollectorOnTcp {
    shutdown: Arc<AtomicBool>,
    listener: TcpListener,
}

impl CollectorOnTcp {
    /// Creates a new TCP execution collector.
    ///
    /// The collector listens to a random port on the loopback interface.
    /// The address of the collector can be obtained by the `address` method.
    pub fn new() -> Result<(Self, SocketAddr), std::io::Error> {
        let shutdown = Arc::new(AtomicBool::new(false));
        // Try IPv4 loopback first, fall back to IPv6 loopback if IPv4 is unavailable.
        let listener = TcpListener::bind("127.0.0.1:0").or_else(|_| TcpListener::bind("[::1]:0"))?;
        let address = listener.local_addr()?;

        Ok((Self { shutdown, listener }, address))
    }

    /// Single-threaded implementation of the collector.
    ///
    /// The collector listens to the TCP port and accepts incoming connections.
    /// When a connection is accepted, the collector reads the execution from
    /// the connection and emits it.
    ///
    /// # Graceful shutdown
    ///
    /// Reporters open a fresh connection per execution and write the whole
    /// length-prefixed record synchronously before the intercepted process
    /// execs. By the time a build finishes, every such report has completed
    /// its TCP handshake and is sitting in the listener's accept backlog.
    ///
    /// When `shutdown` is requested, the backlog may therefore still hold
    /// legitimate, fully-written reports. To avoid losing them, once the
    /// shutdown flag is observed the iterator switches the listener to
    /// non-blocking mode and drains every queued connection (reading each
    /// real record) until the backlog is empty (`WouldBlock`), and only then
    /// returns `None`. The drain always terminates because the backlog is
    /// finite and no new connections arrive after the build has exited.
    ///
    /// The throwaway "wake" connection opened by `shutdown` writes no bytes,
    /// so its first read yields end-of-file; it is recognised and skipped
    /// rather than being reported as an error.
    pub fn executions(&self) -> impl Iterator<Item = Result<Execution, ReporterError>> + '_ {
        let listener = &self.listener;
        let shutdown = &self.shutdown;
        // Once true, we are draining the backlog with non-blocking accepts.
        let mut draining = false;

        std::iter::from_fn(move || {
            loop {
                if !draining && shutdown.load(Ordering::Acquire) {
                    // Enter drain mode: stop blocking on accept so the loop is
                    // guaranteed to terminate, and pick up any connections the
                    // reporters already queued before shutdown was requested.
                    if listener.set_nonblocking(true).is_err() {
                        return None;
                    }
                    draining = true;
                }

                match listener.accept() {
                    Ok((mut connection, _)) => {
                        // Read the record before closing. A connection that
                        // does not deliver a complete record (the wake
                        // connection from `shutdown`, or any aborted report) is
                        // skipped rather than surfaced as an error.
                        let execution = Self::read_connection(&mut connection);
                        let _ = connection.shutdown(std::net::Shutdown::Both);
                        match execution {
                            Some(execution) => return Some(Ok(execution)),
                            None => continue,
                        }
                    }
                    Err(err) if draining && err.kind() == std::io::ErrorKind::WouldBlock => {
                        // Backlog fully drained: nothing left to accept.
                        return None;
                    }
                    Err(err) => return Some(Err(ReporterError::Network(err))),
                }
            }
        })
    }

    /// Reads a single execution from an accepted connection, if the peer sent
    /// a complete record.
    ///
    /// Returns `None` when the connection does not yield a usable record. The
    /// empty wake connection opened by `shutdown` closes without sending any
    /// bytes - a clean EOF on Linux, but a reset or other platform-specific
    /// error on macOS/BSD - and an aborted or truncated report fails partway
    /// through. None of these are data events; reporting failures are
    /// non-fatal by contract, so such connections are skipped (logged at
    /// debug) rather than surfaced as collector errors. Keying on "did the
    /// peer deliver a complete record" instead of on a specific I/O error kind
    /// keeps the behavior identical across platforms.
    fn read_connection(connection: &mut TcpStream) -> Option<Execution> {
        match ExecutionWireSerializer::read(connection) {
            Ok(execution) => Some(execution),
            Err(err) => {
                log::debug!("Skipping connection without a complete record: {err}");
                None
            }
        }
    }

    /// Stops the collector by flipping the shutdown flag and connecting to the collector.
    ///
    /// The collector is stopped when the `executions` iterator sees the shutdown
    /// flag. To signal the collector to stop, we connect to it to unblock the
    /// blocking `accept` call so it can observe the flag and drain the backlog.
    pub fn shutdown(&self) -> Result<(), ReporterError> {
        self.shutdown.store(true, Ordering::Release);

        let address = self.listener.local_addr()?;
        let _ = TcpStream::connect(address).map_err(ReporterError::Network)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intercept::reporter::Reporter;
    use intercept::tcp::ReporterOnTcp;
    use std::collections::HashMap;

    fn executions() -> Vec<Execution> {
        vec![
            Execution::from_strings("/usr/bin/ls", vec!["ls", "-l"], "/tmp", HashMap::new()),
            Execution::from_strings(
                "/usr/bin/cc",
                vec!["cc", "-c", "./file_a.c", "-o", "./file_a.o"],
                "/home/user",
                HashMap::from([("PATH", "/usr/bin:/bin"), ("CC", "gcc")]),
            ),
            Execution::from_strings(
                "/usr/bin/ld",
                vec!["ld", "-o", "./file_a", "./file_a.o"],
                "/opt/project",
                HashMap::from([("PATH", "/usr/bin:/bin"), ("LD_PRELOAD", "/usr/lib:/lib")]),
            ),
        ]
    }

    // Test that the TCP reporter and the TCP collector work together.
    // We create a TCP collector and a TCP reporter, then we send executions
    // to the reporter and check if the collector receives them.
    //
    // A channel is used so the collector thread can signal that all
    // executions have been received, and a timeout prevents the test from
    // hanging indefinitely if delivery is broken.
    #[test]
    fn tcp_reporter_and_collectors_work() {
        let expected = executions();
        let (collector, address) = CollectorOnTcp::new().unwrap();
        let collector_arc = Arc::new(collector);

        // Channel for the collector to signal "I've received everything"
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);

        // Start the collector in a separate thread using the executions iterator
        let collector_thread = {
            let tcp_collector = Arc::clone(&collector_arc);
            let count = expected.len();
            std::thread::spawn(move || {
                let mut received = Vec::new();
                for result in tcp_collector.executions() {
                    match result {
                        Ok(execution) => {
                            received.push(execution);
                            if received.len() == count {
                                let _ = done_tx.send(());
                                break;
                            }
                        }
                        Err(err) => {
                            log::error!("cannot receive an intercepted command: {err}");
                            break;
                        }
                    }
                }
                received
            })
        };

        // Send executions to the reporter.
        for execution in expected.iter() {
            let reporter = ReporterOnTcp::new(address);
            let result = reporter.report(execution.clone());
            assert!(result.is_ok());
        }

        // Wait with a timeout — if delivery is broken, fail instead of hang.
        done_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("timed out waiting for collector to receive all executions");

        // Now safe to shutdown and join.
        collector_arc.shutdown().unwrap();
        let received = collector_thread.join().unwrap();

        // Assert that we received all the executions.
        assert_eq!(expected.len(), received.len());
        for execution in received {
            assert!(expected.contains(&execution));
        }
    }

    // Regression test for issue #704: reports already queued in the accept
    // backlog must not be dropped when shutdown is requested.
    //
    // This reproduces the shutdown race deterministically without relying on
    // arch-specific timing: all reports are sent and fully flushed *before*
    // `shutdown` is called and *before* the collector starts accepting. The
    // reports therefore sit in the listener's accept backlog at the moment
    // shutdown flips the flag, exactly as the last compiler's report does at
    // the end of a real build. A correct collector drains the backlog and
    // yields every queued execution; the pre-fix collector discarded the
    // backlog and lost them.
    //
    // Requirements: interception-preload-mechanism
    #[test]
    fn tcp_collector_drains_backlog_on_shutdown() {
        let expected = executions();
        let (collector, address) = CollectorOnTcp::new().unwrap();
        let collector_arc = Arc::new(collector);

        // Send and fully flush every report up front. Because the reporter
        // opens a connection, writes the whole record, and closes, each report
        // ends up queued in the accept backlog before we ever accept.
        for execution in expected.iter() {
            let reporter = ReporterOnTcp::new(address);
            reporter.report(execution.clone()).expect("report should succeed");
        }

        // Request shutdown while every report is still sitting in the backlog.
        collector_arc.shutdown().unwrap();

        // Drain on a worker thread guarded by a timeout so a regression that
        // hangs (instead of dropping) is reported as a failure, not a hang.
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<Vec<Execution>>(0);
        let drain_thread = {
            let tcp_collector = Arc::clone(&collector_arc);
            std::thread::spawn(move || {
                let received: Vec<Execution> = tcp_collector.executions().filter_map(Result::ok).collect();
                let _ = done_tx.send(received);
            })
        };

        let received = done_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("timed out draining the shutdown backlog");
        drain_thread.join().unwrap();

        // Every queued report must survive shutdown.
        assert_eq!(expected.len(), received.len());
        for execution in expected.iter() {
            assert!(received.contains(execution));
        }
    }

    // Test that calling shutdown on the collector stops the executions
    // iterator. No data is sent — this purely tests the shutdown mechanism.
    #[test]
    fn tcp_collector_shutdown_stops_iterator() {
        let (collector, _address) = CollectorOnTcp::new().unwrap();
        let collector_arc = Arc::new(collector);

        let collector_thread = {
            let tcp_collector = Arc::clone(&collector_arc);
            std::thread::spawn(move || tcp_collector.executions().count())
        };

        collector_arc.shutdown().unwrap();

        let count = collector_thread.join().unwrap();
        assert_eq!(count, 0);
    }
}

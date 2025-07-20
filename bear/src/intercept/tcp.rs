// SPDX-License-Identifier: GPL-3.0-or-later

//! The module contains the implementation of the TCP collector and reporter.

use super::collector::{Cancellable, CancellableProducer, CollectorError, Producer};
use super::reporter::{Reporter, ReportingError};
use super::Event;
use crossbeam_channel::Sender;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// The serializer for events to transmit over the network.
///
/// The events are serialized using TLV (Type-Length-Value) format.
/// The type is always 0, the length is a 4-byte big-endian integer,
/// and the value is the JSON representation of the event.
struct EventWireSerializer;

impl EventWireSerializer {
    /// Read an event from a reader using TLV format.
    fn read(reader: &mut impl Read) -> Result<Event, ReceivingError> {
        let mut length_bytes = [0; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_be_bytes(length_bytes) as usize;

        let mut buffer = vec![0; length];
        reader.read_exact(&mut buffer)?;
        let event = serde_json::from_slice(buffer.as_ref())?;

        Ok(event)
    }

    /// Write an event to a writer using TLV format.
    fn write(writer: &mut impl Write, event: Event) -> Result<u32, ReportingError> {
        let serialized_event = serde_json::to_string(&event)?;
        let bytes = serialized_event.into_bytes();
        let length = bytes.len() as u32;

        writer.write_all(&length.to_be_bytes())?;
        writer.write_all(&bytes)?;

        Ok(length)
    }
}

/// Errors that can occur in the collector.
#[derive(Error, Debug)]
pub enum ReceivingError {
    #[error("Receiving event failed with IO error: {0}")]
    Network(#[from] std::io::Error),
    #[error("Receiving event failed with serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Represents a TCP event collector.
pub struct CollectorOnTcp {
    shutdown: Arc<AtomicBool>,
    listener: TcpListener,
}

impl CollectorOnTcp {
    /// Creates a new TCP event collector.
    ///
    /// The collector listens to a random port on the loopback interface.
    /// The address of the collector can be obtained by the `address` method.
    pub fn new() -> Result<(Self, SocketAddr), std::io::Error> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;

        Ok((Self { shutdown, listener }, address))
    }
}

impl Producer<Event, CollectorError> for CollectorOnTcp {
    /// Single-threaded implementation of the collector.
    ///
    /// The collector listens to the TCP port and accepts incoming connections.
    /// When a connection is accepted, the collector reads the events from the
    /// connection and sends them to the destination channel.
    fn produce(&self, destination: Sender<Event>) -> Result<(), CollectorError> {
        for stream in self.listener.incoming() {
            // This has to be the first thing to do, to implement the stop method!
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            match stream {
                Ok(mut connection) => {
                    // ... (process the connection in a separate thread or task)
                    let event = EventWireSerializer::read(&mut connection);
                    match event {
                        Ok(event) => {
                            // Send the event to the destination channel
                            destination
                                .send(event)
                                .map_err(|err| CollectorError::Channel(err.to_string()))?;
                        }
                        Err(err) => {
                            // Log the error and continue to the next connection
                            log::error!("Failed to read event: {err}");
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connection available, continue checking for shutdown
                    continue;
                }
                Err(err) => {
                    log::error!("Error while reading the socket: {err}");
                    break;
                }
            }
        }
        Ok(())
    }
}

impl Cancellable<CollectorError> for CollectorOnTcp {
    /// Stops the collector by flipping the shutdown flag and connecting to the collector.
    ///
    /// The collector is stopped when the `produce` method sees the shutdown flag.
    /// To signal the collector to stop, we connect to the collector to unblock the
    /// `accept` call to check the shutdown flag.
    fn cancel(&self) -> Result<(), CollectorError> {
        self.shutdown.store(true, Ordering::Relaxed);

        let address = self.listener.local_addr()?;
        let _ = TcpStream::connect(address).map_err(CollectorError::Network)?;
        Ok(())
    }
}

impl CancellableProducer<Event, CollectorError> for CollectorOnTcp {}

/// Represents a TCP event reporter.
pub struct ReporterOnTcp {
    destination: String,
}

impl ReporterOnTcp {
    /// Creates a new TCP reporter instance.
    ///
    /// It does not open the TCP connection yet. Stores the destination
    /// address and creates a unique reporter id.
    pub fn new(destination: String) -> Self {
        Self { destination }
    }
}

impl Reporter for ReporterOnTcp {
    /// Sends an event to the remote collector.
    ///
    /// The event is wrapped in an envelope and sent to the remote collector.
    /// The TCP connection is opened and closed for each event.
    fn report(&self, event: Event) -> Result<(), ReportingError> {
        let mut socket =
            TcpStream::connect(self.destination.clone()).map_err(ReportingError::Network)?;
        EventWireSerializer::write(&mut socket, event)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::Arc;
    use std::thread;

    // Test that the serialization and deserialization of the Envelope works.
    // We write the Envelope to a buffer and read it back to check if the
    // deserialized Envelope is the same as the original one.
    #[test]
    fn read_write_works() {
        let mut writer = Cursor::new(vec![0; 1024]);
        for event in fixtures::EVENTS.iter() {
            let result = EventWireSerializer::write(&mut writer, event.clone());
            assert!(result.is_ok());
        }

        let mut reader = Cursor::new(writer.get_ref());
        for event in fixtures::EVENTS.iter() {
            let result = EventWireSerializer::read(&mut reader);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), event.clone());
        }
    }

    // Test that the TCP reporter and the TCP collector work together.
    // We create a TCP collector and a TCP reporter, then we send events
    // to the reporter and check if the collector receives them.
    //
    // We use a bounded channel to send the events from the reporter to the
    // collector. The collector reads the events from the channel and checks
    // if they are the same as the original events.
    #[test]
    fn tcp_reporter_and_collectors_work() {
        let (input, output) = crossbeam_channel::unbounded();

        let (collector, address) = CollectorOnTcp::new().unwrap();
        let collector_arc = Arc::new(collector);
        let reporter = ReporterOnTcp::new(address.to_string());

        // Start a consumer thread to collect events from the output channel
        let drain_thread = thread::spawn(move || {
            let mut events = Vec::new();
            for event in output.iter() {
                events.push(event);
                if events.len() == fixtures::EVENTS.len() {
                    break;
                }
            }
            events
        });

        // Start the collector in a separate thread.
        let collector_thread = {
            let tcp_collector = Arc::clone(&collector_arc);
            thread::spawn(move || {
                tcp_collector.produce(input).unwrap();
            })
        };

        // Send events to the reporter.
        for event in fixtures::EVENTS.iter() {
            let result = reporter.report(event.clone());
            assert!(result.is_ok());
        }

        // Call the stop method to stop the collector.
        {
            let tcp_collector = Arc::clone(&collector_arc);
            tcp_collector.cancel().unwrap();
        }

        // Wait for all events to be consumed
        let received_events = drain_thread.join().unwrap();

        // Assert that we received all the events.
        assert_eq!(received_events.len(), fixtures::EVENTS.len());
        for event in received_events {
            assert!(fixtures::EVENTS.contains(&event));
        }

        // shutdown the receiver thread
        collector_thread.join().unwrap();
    }

    mod fixtures {
        use super::*;
        use std::collections::HashMap;

        pub(super) static EVENTS: std::sync::LazyLock<Vec<Event>> =
            std::sync::LazyLock::new(|| {
                vec![
                    Event::from_strings(
                        3425,
                        "/usr/bin/ls",
                        vec!["ls", "-l"],
                        "/tmp",
                        HashMap::new(),
                    ),
                    Event::from_strings(
                        3492,
                        "/usr/bin/cc",
                        vec!["cc", "-c", "./file_a.c", "-o", "./file_a.o"],
                        "/home/user",
                        HashMap::from([("PATH", "/usr/bin:/bin"), ("HOME", "/home/user")]),
                    ),
                    Event::from_strings(
                        3522,
                        "/usr/bin/ld",
                        vec!["ld", "-o", "./file_a", "./file_a.o"],
                        "/opt/project",
                        HashMap::from([
                            ("PATH", "/usr/bin:/bin"),
                            ("LD_LIBRARY_PATH", "/usr/lib:/lib"),
                        ]),
                    ),
                ]
            });
    }
}

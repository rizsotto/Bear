/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::net::{SocketAddr, TcpListener, TcpStream};

use crossbeam::channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::Envelope;

/// Represents the local sink of supervised process events.
///
/// The collector is responsible for collecting the events from the reporters.
///
/// To share the collector between threads, we use the `Arc` type to wrap the
/// collector. This way we can clone the collector and send it to other threads.
pub trait EventCollector {
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

pub struct EventCollectorOnTcp {
    shutdown: Arc<AtomicBool>,
    listener: TcpListener,
    address: SocketAddr,
}

impl EventCollectorOnTcp {
    /// Creates a new TCP event collector.
    ///
    /// The collector listens on a random port on the loopback interface.
    /// The address of the collector can be obtained by the `address` method.
    pub fn new() -> Result<Self, anyhow::Error> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;

        let result = EventCollectorOnTcp {
            shutdown,
            listener,
            address,
        };

        Ok(result)
    }

    fn send(
        &self,
        mut socket: TcpStream,
        destination: Sender<Envelope>,
    ) -> Result<(), anyhow::Error> {
        let envelope = Envelope::read_from(&mut socket)?;
        destination.send(envelope)?;

        Ok(())
    }
}

impl EventCollector for EventCollectorOnTcp {
    fn address(&self) -> String {
        self.address.to_string()
    }

    /// Single-threaded implementation of the collector.
    ///
    /// The collector listens on the TCP port and accepts incoming connections.
    /// When a connection is accepted, the collector reads the events from the
    /// connection and sends them to the destination channel.
    fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error> {
        for stream in self.listener.incoming() {
            // This has to be the first thing to do, in order to implement the stop method!
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            match stream {
                Ok(connection) => {
                    // ... (process the connection in a separate thread or task)
                    self.send(connection, destination.clone())?;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No new connection available, continue checking for shutdown
                    continue;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    /// Stops the collector by flipping the shutdown flag and connecting to the collector.
    ///
    /// The collector is stopped when the `collect` method sees the shutdown flag.
    /// To signal the collector to stop, we connect to the collector to unblock the
    /// `accept` call to check the shutdown flag.
    fn stop(&self) -> Result<(), anyhow::Error> {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address)?;
        Ok(())
    }
}

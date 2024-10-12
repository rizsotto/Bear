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
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::Envelope;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SessionLocator(pub String);

pub trait EventCollector {
    fn address(&self) -> SessionLocator;
    fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error>;
    fn stop(&self) -> Result<(), anyhow::Error>;
}

pub struct EventCollectorOnTcp {
    shutdown: Arc<AtomicBool>,
    listener: TcpListener,
    address: SocketAddr,
}

impl EventCollectorOnTcp {
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
    fn address(&self) -> SessionLocator {
        SessionLocator(self.address.to_string())
    }

    fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error> {
        for stream in self.listener.incoming() {
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

    fn stop(&self) -> Result<(), anyhow::Error> {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address)?;
        Ok(())
    }
}

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

use std::net::{TcpListener, TcpStream};

use crossbeam::channel::{Receiver, Sender};
use crossbeam_channel::bounded;
use serde::{Deserialize, Serialize};

use super::Envelope;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SessionLocator(pub String);

pub trait EventCollector {
    fn address(&self) -> Result<SessionLocator, anyhow::Error>;
    fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error>;
    fn stop(&self) -> Result<(), anyhow::Error>;
}

pub struct EventCollectorOnTcp {
    control_input: Sender<bool>,
    control_output: Receiver<bool>,
    listener: TcpListener,
}

impl EventCollectorOnTcp {
    pub fn new() -> Result<Self, anyhow::Error> {
        let (control_input, control_output) = bounded(0);
        let listener = TcpListener::bind("127.0.0.1:0")?;

        let result = EventCollectorOnTcp {
            control_input,
            control_output,
            listener,
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
    fn address(&self) -> Result<SessionLocator, anyhow::Error> {
        let local_addr = self.listener.local_addr()?;
        let locator = SessionLocator(local_addr.to_string());
        Ok(locator)
    }

    fn collect(&self, destination: Sender<Envelope>) -> Result<(), anyhow::Error> {
        loop {
            if let Ok(shutdown) = self.control_output.try_recv() {
                if shutdown {
                    break;
                }
            }

            match self.listener.accept() {
                Ok((stream, _)) => {
                    println!("Got a connection");
                    // ... (process the connection in a separate thread or task)
                    self.send(stream, destination.clone())?;
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

        println!("Server shutting down");
        Ok(())
    }

    fn stop(&self) -> Result<(), anyhow::Error> {
        self.control_input.send(true)?;
        Ok(())
    }
}

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

use std::net::TcpStream;

use rand::random;

use super::{Envelope, Event, ReporterId};

impl ReporterId {
    pub fn new() -> Self {
        let id = random::<u64>();
        ReporterId(id)
    }
}

// Represents the remote sink of supervised process events.
//
// Events from a process execution can be sent from many actors (mostly
// supervisor processes). The events are collected in a common place
// in order to reconstruct of final report of a build process.
pub trait Reporter {
    fn report(&mut self, event: Event) -> Result<(), anyhow::Error>;
}

struct TcpReporter {
    socket: TcpStream,
    destination: String,
    reporter_id: ReporterId,
}

impl TcpReporter {
    pub fn new(destination: String) -> Result<Self, anyhow::Error> {
        let socket = TcpStream::connect(destination.clone())?;
        let reporter_id = ReporterId::new();
        let result = TcpReporter { socket, destination, reporter_id };
        Ok(result)
    }
}

impl Reporter for TcpReporter {
    fn report(&mut self, event: Event) -> Result<(), anyhow::Error> {
        let envelope = Envelope::new(&self.reporter_id, event);
        envelope.write_into(&mut self.socket)?;

        Ok(())
    }
}

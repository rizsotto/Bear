/*  Copyright (C) 2012-2018 by László Nagy
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

pub type ExitCode = i32;
pub type DateTime = chrono::DateTime<chrono::Utc>;
pub type ProcessId = u32;
pub type SignalId = String;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Created {
        when: DateTime,
        pid: ProcessId,
        ppid: ProcessId,
        cwd: std::path::PathBuf,
        program: std::path::PathBuf,
        args: Vec<String>,
    },
    TerminatedNormally {
        when: DateTime,
        pid: ProcessId,
        code: ExitCode,
    },
    TerminatedAbnormally {
        when: DateTime,
        pid: ProcessId,
        signal: SignalId,
    },
    Stopped {
        when: DateTime,
        pid: ProcessId,
        signal: SignalId,
    },
    Continued {
        when: DateTime,
        pid: ProcessId,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventEnvelope {
    id: ProcessId,
    at: DateTime,
    content: Event,
}

impl EventEnvelope {
    pub fn new(id: ProcessId, content: Event) -> Self {
        let at = chrono::Utc::now();
        EventEnvelope { id, at, content }
    }

    pub fn pid(&self) -> ProcessId {
        self.id
    }

    pub fn to_execution(&self) -> Option<(Vec<String>, std::path::PathBuf)> {
        match self.content {
            Event::Created { ref args, ref cwd, .. } =>
                Some((args.to_vec(), cwd.to_path_buf())),
            _ =>
                None,
        }
    }
}

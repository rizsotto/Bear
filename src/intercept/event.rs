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

use std::env;
use crate::intercept::{Result, ResultExt};

pub type ExitCode = i32;
pub type DateTime = chrono::DateTime<chrono::Utc>;
pub type ProcessId = u32;
pub type SignalId = String;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Created {
        ppid: ProcessId,
        cwd: std::path::PathBuf,
        program: std::path::PathBuf,
        args: Vec<String>,
    },
    TerminatedNormally {
        code: ExitCode,
    },
    TerminatedAbnormally {
        signal: SignalId,
    },
    Stopped {
        signal: SignalId,
    },
    Continued {
    },
}

impl Event {
    pub fn created(program: &std::path::Path, args: &[String]) -> Result<Event> {
        let cwd = env::current_dir()
            .chain_err(|| "Unable to get current working directory")?;

        let result = Event::Created {
            ppid: inner::get_parent_pid(),
            cwd,
            program: program.to_path_buf(),
            args: args.to_vec()
        };

        Ok(result)
    }
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

    pub fn event(&self) -> &Event {
        &self.content
    }

    pub fn to_execution(&self) -> Option<(Vec<String>, std::path::PathBuf)> {
        match self.content {
            Event::Created { ref args, ref cwd, .. } =>
                Some((args.to_vec(), cwd.to_path_buf())),
            _ =>
                None,
        }
    }

    #[cfg(test)]
    pub fn create(id: ProcessId, at: DateTime, content: Event) -> Self {
        EventEnvelope { id, at, content }
    }
}

mod inner {
    use super::ProcessId;

    #[cfg(unix)]
    pub fn get_parent_pid() -> ProcessId {
        std::os::unix::process::parent_id()
    }

    #[cfg(not(unix))]
    pub fn get_parent_pid() -> ProcessId {
        use crate::intercept::inner::env;

        env::get::parent_pid()
            .unwrap_or(0)
    }
}

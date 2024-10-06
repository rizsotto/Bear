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

mod fixtures;
pub mod tools;

use intercept::ipc::Execution;
use std::path::PathBuf;

/// Represents a semantic recognition result.
#[derive(Debug, PartialEq)]
pub enum RecognitionResult {
    Recognized(Result<Meaning, String>),
    NotRecognized,
}

/// Represents an executed command semantic.
#[derive(Debug, PartialEq)]
pub enum Meaning {
    /// This is a compiler call.
    Compiler {
        compiler: PathBuf,
        working_dir: PathBuf,
        passes: Vec<CompilerPass>,
    },
    /// This is something else we recognised, but not interested to fully specify.
    Ignored,
}

/// Represents a compiler call.
#[derive(Debug, PartialEq)]
pub enum CompilerPass {
    Preprocess,
    Compile {
        source: PathBuf,
        output: Option<PathBuf>,
        flags: Vec<String>,
    },
}

/// This abstraction is representing a tool which semantic we are aware of.
///
/// A single tool has a potential to recognize a command execution and
/// identify the semantic of that command. This abstraction is also can
/// represent a set of tools, and the recognition process can be distributed
/// amongst the tools.
pub trait Tool: Send {
    fn recognize(&self, _: &Execution) -> RecognitionResult;
}

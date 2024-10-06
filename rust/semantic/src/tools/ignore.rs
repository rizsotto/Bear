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

use std::path::PathBuf;
use intercept::ipc::Execution;
use crate::{RecognitionResult, Meaning, Tool};


pub struct IgnoreByPath {
    executables: Vec<PathBuf>,
}

impl IgnoreByPath {
    pub fn new(compilers: &[PathBuf]) -> Box<dyn Tool> {
        let executables = compilers.iter()
            .map(|compiler| compiler.clone())
            .collect();
        Box::new(Self { executables })
    }
}

impl Tool for IgnoreByPath {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        if self.executables.contains(&execution.executable) {
            RecognitionResult::Recognized(Ok(Meaning::Ignored))
        } else {
            RecognitionResult::NotRecognized
        }
    }
}


pub struct IgnoreByArgs {
    args: Vec<String>,
}

impl IgnoreByArgs {
    pub fn new(args: &[String]) -> Box<dyn Tool> {
        let clones = args.iter().map(|arg| arg.clone()).collect();
        Box::new(Self { args: clones })
    }
}

impl Tool for IgnoreByArgs {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        if execution.arguments.iter().any(|arg| self.args.contains(arg)) {
            RecognitionResult::Recognized(Ok(Meaning::Ignored))
        } else {
            RecognitionResult::NotRecognized
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // TODO: implement test cases
}
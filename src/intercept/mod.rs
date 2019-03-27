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

pub mod environment;
pub mod event;
pub mod protocol;
pub mod supervisor;

use crate::Result;

pub type Environment = std::collections::HashMap<String, String>;

pub struct EnvironmentBuilder {}

impl EnvironmentBuilder {

    fn new() -> EnvironmentBuilder {
        unimplemented!()
    }

    fn from(_environment: &Environment) -> EnvironmentBuilder {
        unimplemented!()
    }

    fn build(&self) -> Environment {
        unimplemented!()
    }

    fn with_mode(&mut self, _mode: &InterceptMode) -> &mut EnvironmentBuilder {
        unimplemented!()
    }

    fn with_modes(&mut self, modes: &[InterceptMode]) -> &mut EnvironmentBuilder {
        for mode in modes {
            self.with_mode(mode);
        }
        self
    }

    fn with_verbose(&mut self, _verbose: bool) -> &mut EnvironmentBuilder {
        unimplemented!()
    }

    fn with_destination(&mut self, _destination: &std::path::Path) -> &mut EnvironmentBuilder {
        unimplemented!()
    }
}

pub struct Executor {}

impl Executor {

    fn new(_sink: std::sync::mpsc::Sender<event::Event>) -> Executor {
        unimplemented!()
    }

    fn intercept(_execution: &Execution, _environment: &Environment) -> Result<ExitCode>
    {
        // set environment
        // execute command
        // collect and send events
        unimplemented!()
    }

    fn supervise(_execution: &Execution, _environment: &Environment) -> Result<ExitCode>
    {
        // set environment
        // execute command
        // send events
        unimplemented!()
    }

    fn fake(_execution: &Execution) -> Result<ExitCode>
    {
        // send events
        unimplemented!()
    }
}

pub type ExitCode = i32;

#[derive(Debug, PartialEq, Eq)]
pub enum InterceptMode {
    Library(std::path::PathBuf),
    Wrapper(String, std::path::PathBuf),
}

pub type InterceptModes = Vec<InterceptMode>;

#[derive(Debug, PartialEq, Eq)]
pub struct Execution {
    pub program: ExecutionTarget,
    pub arguments: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionTarget {
    ByFilename(std::path::PathBuf),
    WithPath(String),
    WithSearchPath(String, Vec<std::path::PathBuf>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub destination: std::path::PathBuf,
    pub library: std::path::PathBuf,
    pub verbose: bool,
}

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

pub type ExitCode = i32;

#[derive(Debug, PartialEq, Eq)]
pub enum InterceptMode {
    Library(std::path::PathBuf),
    Wrapper(String, std::path::PathBuf),
}

pub type InterceptModes = Vec<InterceptMode>;

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub executable: Executable,
    pub arguments: Vec<String>,
}

impl ExecutionRequest {

    fn from_arguments(arguments: &[String]) -> Result<ExecutionRequest> {
        unimplemented!()
    }

    fn from_spec(executable: &Executable, arguments: &[String]) -> Result<ExecutionRequest> {
        unimplemented!()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Executable {
    WithFilename(std::path::PathBuf),
    WithPath(String),
    WithSearchPath(String, Vec<std::path::PathBuf>),
}

impl Executable {

    fn to_absolute_path(&self) -> Result<std::path::PathBuf> {
        unimplemented!()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub destination: std::path::PathBuf,
    pub verbose: bool,
    pub modes: InterceptModes,
}

impl Session {

    fn to_environment(&self) -> Result<environment::Environment> {
        unimplemented!()
    }
}


mod inner {
    use super::*;

    pub struct Executor {}

    impl Executor {
        fn new(_sink: std::sync::mpsc::Sender<event::Event>) -> Executor {
            unimplemented!()
        }

        fn intercept(_execution: &ExecutionRequest, _environment: &environment::Environment) -> Result<ExitCode>
        {
            // set environment
            // execute command
            // collect and send events
            unimplemented!()
        }

        fn supervise(_execution: &ExecutionRequest, _environment: &environment::Environment) -> Result<ExitCode>
        {
            // set environment
            // execute command
            // send events
            unimplemented!()
        }

        fn fake(_execution: &ExecutionRequest) -> Result<ExitCode>
        {
            // send events
            unimplemented!()
        }
    }
}

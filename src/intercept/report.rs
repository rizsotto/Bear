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

use super::*;

pub fn c_compiler(_args: &[String]) -> Result<ExitCode, Error> {

//    let target = environment::target_directory()?;
//    let mut protocol = Protocol::new(target.as_path())?;
//
//    let mut supervisor = Supervisor::new(|event| protocol.send(event));
//
//    match environment::c_compiler_path() {
//        Ok(wrapper) => {
//            args[0] = wrapper;
//            supervisor.run(&args[..])
//        },
//        Err(_) => {
//            supervisor.fake(&args[..])
//        },
//    }

    unimplemented!()
}

pub fn cxx_compiler(_args: &[String]) -> Result<ExitCode, Error> {

//    let target = environment::target_directory()?;
//    let mut protocol = Protocol::new(target.as_path())?;
//
//    let mut supervisor = Supervisor::new(|event| protocol.send(event));
//
//    match environment::cxx_compiler_path() {
//        Ok(wrapper) => {
//            args[0] = wrapper;
//            supervisor.run(&args[..])
//        },
//        Err(_) => {
//            supervisor.fake(&args[..])
//        },
//    }

    unimplemented!()
}

pub fn wrapper(_execution: &ExecutionRequest, _session: &Session) -> Result<ExitCode, Error> {
    unimplemented!()
}


#[derive(Debug, PartialEq, Eq)]
pub enum InterceptMode {
    Library(std::path::PathBuf),
    WrapperCC { wrapper: std::path::PathBuf, compiler: std::path::PathBuf },
    WrapperCXX { wrapper: std::path::PathBuf, compiler: std::path::PathBuf },
}

pub type InterceptModes = Vec<InterceptMode>;

#[derive(Debug, PartialEq, Eq)]
pub struct ExecutionRequest {
    pub executable: Executable,
    pub arguments: Vec<String>,
}

impl ExecutionRequest {

//    fn from_arguments(arguments: &[String]) -> Result<ExecutionRequest> {
//        unimplemented!()
//    }
//
//    fn from_spec(executable: &Executable, arguments: &[String]) -> Result<ExecutionRequest> {
//        unimplemented!()
//    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Executable {
    WithFilename(std::path::PathBuf),
    WithPath(String),
    WithSearchPath(String, Vec<std::path::PathBuf>),
}

impl Executable {

//    fn to_absolute_path(&self) -> Result<std::path::PathBuf> {
//        unimplemented!()
//    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub destination: std::path::PathBuf,
    pub verbose: bool,
    pub modes: InterceptModes,
}

impl Session {

//    fn to_environment(&self) -> Result<environment::Environment> {
//        unimplemented!()
//    }
}

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

pub mod event;
pub mod protocol;
pub mod supervisor;

pub type ExitCode = i32;

#[derive(Debug, PartialEq, Eq)]
pub enum InterceptMode {
    Library(std::path::PathBuf),
    Wrapper(String, std::path::PathBuf),
}

pub type InterceptModes = Vec<InterceptMode>;


#[derive(Debug, PartialEq, Eq)]
pub struct Session {
    pub destination: std::path::PathBuf,
    pub library: std::path::PathBuf,
    pub verbose: bool,
}

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

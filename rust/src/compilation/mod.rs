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

pub mod pass;
pub mod flags;
pub mod database;
pub mod execution;
pub mod compiler;

use crate::Result;

#[derive(Debug)]
pub struct CompilerCall {
    pub work_dir: std::path::PathBuf,
    pub compiler: CompilerExecutable,
    pub flags: Vec<CompilerFlag>,
}

impl CompilerCall {

    pub fn from(_cmd: &[String], _cwd: &std::path::Path) -> Result<CompilerCall> {
        unimplemented!()
    }

    pub fn compiler(&self) -> &CompilerExecutable {
        &self.compiler
    }

    pub fn pass(&self) -> pass::CompilerPass {
        unimplemented!()
    }

    pub fn flags(&self) -> Vec<String> {
        unimplemented!()
    }

    pub fn sources(&self) -> Vec<std::path::PathBuf> {
        unimplemented!()
    }

    pub fn output(&self) -> Option<std::path::PathBuf> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub enum CompilerExecutable {
    CompilerC { path: std::path::PathBuf },
    CompilerCxx { path: std::path::PathBuf },
    Wrapper { path: std::path::PathBuf, compiler: Option<Box<CompilerExecutable>> },
}

impl CompilerExecutable {

    pub fn as_vec(&self, drop_wrapper: bool) -> Vec<std::path::PathBuf> {
        match self {
            CompilerExecutable::CompilerC { path, .. } => vec!(path.to_path_buf()),
            CompilerExecutable::CompilerCxx { path, .. } => vec!(path.to_path_buf()),
            CompilerExecutable::Wrapper { path, compiler, .. } => {
                match compiler {
                    Some(c) if !drop_wrapper => {
                        let mut result = vec!(path.to_path_buf());
                        result.extend(c.as_ref().as_vec(drop_wrapper));
                        result
                    },
                    Some(c) => c.as_ref().as_vec(drop_wrapper),
                    None => vec!(path.to_path_buf()),
                }
            },
        }
    }

    pub fn to_strings(&self) -> Vec<String> {
        self.as_vec(false)
            .iter()
            .map(|path: &std::path::PathBuf| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    }
}

#[derive(Debug)]
pub enum CompilerFlag {
    Pass { pass: pass::CompilerPass },
    Preprocessor { },
    Linker { },
    Output { },
    Source { },
    Other { },
    Ignored { },
}

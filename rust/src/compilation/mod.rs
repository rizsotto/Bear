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

pub struct CompilerCall {
    work_dir: std::path::PathBuf,
    compiler: CompilerExecutable,
    flags: Vec<CompilerFlag>,
}

pub enum CompilerExecutable {
    CompilerC { path: std::path::PathBuf },
    CompilerCxx { path: std::path::PathBuf },
    Wrapper { compiler: std::boxed::Box<CompilerExecutable> },
}

pub enum CompilerFlag {
    Pass { pass: pass::CompilerPass },
    Preprocessor { },
    Linker { },
    Output { },
    Source { },
    Other { },
    Ignored { },
}

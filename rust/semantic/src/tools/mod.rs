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

use super::tools::combinators::Any;
use super::tools::generic::Generic;
use super::tools::ignore::{IgnoreByArgs, IgnoreByPath};
use super::Tool;

mod combinators;
mod gcc;
mod generic;
mod ignore;
mod matchers;

pub struct Builder {
    tools: Vec<Box<dyn Tool>>,
}

// TODO: write unit test for this!!!
impl Builder {
    pub fn new() -> Self {
        // FIXME: replace this with gcc, when it's implemented
        let gcc = PathBuf::from("/usr/bin/g++");
        Builder {
            tools: vec![
                // ignore executables which are not compilers,
                IgnoreByPath::new(),
                // recognize default compiler
                Generic::new(gcc.as_path()),
            ],
        }
    }

    pub fn build(self) -> impl Tool {
        Any::new(self.tools)
    }

    pub fn compilers_to_recognize(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add the new compilers at the end of the tools.
            for compiler in compilers {
                let tool = Generic::new(compiler);
                self.tools.push(tool);
            }
        }
        self
    }

    pub fn compilers_to_exclude(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add these new compilers at the front of the tools.
            let tool = IgnoreByPath::from(compilers);
            self.tools.insert(0, tool);
        }
        self
    }

    pub fn compilers_to_exclude_by_arguments(mut self, args: &[String]) -> Self {
        if !args.is_empty() {
            // Add these new compilers at the front of the tools.
            let tool = IgnoreByArgs::new(args);
            self.tools.insert(0, tool);
        }
        self
    }
}

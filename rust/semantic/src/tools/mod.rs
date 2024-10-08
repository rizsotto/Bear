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

/// A builder for creating a tool which can recognize the semantic of a compiler,
/// or ignore known non-compilers.
pub struct Builder {
    tools: Vec<Box<dyn Tool>>,
}

impl Builder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        // FIXME: replace generic with gcc, when it's implemented
        Builder {
            tools: vec![
                // ignore executables which are not compilers,
                IgnoreByPath::new(),
                // recognize default compiler
                Generic::from(&[PathBuf::from("/usr/bin/g++")]),
            ],
        }
    }

    /// Factory method to create a new tool from the builder.
    pub fn build(self) -> impl Tool {
        Any::new(self.tools)
    }

    /// Adds new tools to recognize as compilers by executable name.
    pub fn compilers_to_recognize(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add the new compilers at the end of the tools.
            let tool = Generic::from(compilers);
            self.tools.push(tool);
        }
        self
    }

    /// Adds new tools to recognize as non-compilers by executable names.
    pub fn compilers_to_exclude(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add these new compilers at the front of the tools.
            let tool = IgnoreByPath::from(compilers);
            self.tools.insert(0, tool);
        }
        self
    }

    /// Adds new tools to recognize as non-compilers by arguments.
    pub fn compilers_to_exclude_by_arguments(mut self, args: &[String]) -> Self {
        if !args.is_empty() {
            // Add these new compilers at the front of the tools.
            let tool = IgnoreByArgs::new(args);
            self.tools.insert(0, tool);
        }
        self
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::{vec_of_pathbuf, vec_of_strings};
    use super::super::{Meaning, RecognitionResult};
    use super::*;
    use intercept::Execution;

    #[test]
    fn test_builder() {
        let sut = Builder::new().build();

        let input = any_execution();
        match sut.recognize(&input) {
            RecognitionResult::Recognized(Ok(Meaning::Compiler { .. })) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_builder_with_compilers_to_exclude() {
        let compilers = vec_of_pathbuf!["/usr/bin/g++"];
        let sut = Builder::new().compilers_to_exclude(&compilers).build();

        let input = any_execution();
        match sut.recognize(&input) {
            RecognitionResult::Recognized(Ok(Meaning::Ignored)) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_builder_with_compilers_to_exclude_by_arguments() {
        let args = vec_of_strings!["-c"];
        let sut = Builder::new()
            .compilers_to_exclude_by_arguments(&args)
            .build();

        let input = any_execution();
        match sut.recognize(&input) {
            RecognitionResult::Recognized(Ok(Meaning::Ignored)) => assert!(true),
            _ => assert!(false),
        }
    }

    fn any_execution() -> Execution {
        Execution {
            executable: PathBuf::from("/usr/bin/g++"),
            arguments: vec_of_strings!["g++", "-c", "main.cpp"],
            environment: HashMap::new(),
            working_dir: PathBuf::from("/home/user"),
        }
    }
}

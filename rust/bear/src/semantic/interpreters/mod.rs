// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use super::interpreters::combinators::Any;
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::Interpreter;

mod combinators;
mod gcc;
mod generic;
mod ignore;
mod matchers;

/// A builder for creating a tool which can recognize the semantic of a compiler,
/// or ignore known non-compilers.
pub struct Builder {
    interpreters: Vec<Box<dyn Interpreter>>,
}

impl Builder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        // FIXME: replace generic with gcc, when it's implemented
        Builder {
            interpreters: vec![
                // ignore executables which are not compilers,
                IgnoreByPath::new(),
                // recognize default compiler
                Generic::from(&[PathBuf::from("/usr/bin/g++")]),
            ],
        }
    }

    /// Factory method to create a new tool from the builder.
    pub fn build(self) -> impl Interpreter {
        Any::new(self.interpreters)
    }

    /// Adds new interpreters to recognize as compilers by executable name.
    pub fn compilers_to_recognize(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add the new compilers at the end of the interpreters.
            let tool = Generic::from(compilers);
            self.interpreters.push(tool);
        }
        self
    }

    /// Adds new interpreters to recognize as non-compilers by executable names.
    pub fn compilers_to_exclude(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add these new compilers at the front of the interpreters.
            let tool = IgnoreByPath::from(compilers);
            self.interpreters.insert(0, tool);
        }
        self
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::{Meaning, Recognition};
    use super::*;
    use crate::{vec_of_pathbuf, vec_of_strings};
    use intercept::Execution;

    #[test]
    fn test_builder() {
        let sut = Builder::new().build();

        let input = any_execution();
        match sut.recognize(&input) {
            Recognition::Success(Meaning::Compiler { .. }) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_builder_with_compilers_to_exclude() {
        let compilers = vec_of_pathbuf!["/usr/bin/g++"];
        let sut = Builder::new().compilers_to_exclude(&compilers).build();

        let input = any_execution();
        match sut.recognize(&input) {
            Recognition::Success(Meaning::Ignored) => assert!(true),
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

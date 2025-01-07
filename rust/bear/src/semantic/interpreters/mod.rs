// SPDX-License-Identifier: GPL-3.0-or-later

use super::interpreters::combinators::Any;
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::Interpreter;
use crate::config;
use std::path::PathBuf;

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
    /// Creates an interpreter to recognize the compiler calls.
    ///
    /// Using the configuration we can define which compilers to include and exclude.
    /// Also read the environment variables to detect the compiler to include (and
    /// make sure those are not excluded either).
    // TODO: Use the CC or CXX environment variables to detect the compiler to include.
    //       Use the CC or CXX environment variables and make sure those are not excluded.
    //       Make sure the environment variables are passed to the method.
    // TODO: Take environment variables as input.
    pub fn from(config: &config::Main) -> impl Interpreter {
        let compilers_to_include = match &config.intercept {
            config::Intercept::Wrapper { executables, .. } => executables.clone(),
            _ => vec![],
        };
        let compilers_to_exclude = match &config.output {
            config::Output::Clang { compilers, .. } => compilers
                .iter()
                .filter(|compiler| compiler.ignore == config::IgnoreOrConsider::Always)
                .map(|compiler| compiler.path.clone())
                .collect(),
            _ => vec![],
        };
        Builder::new()
            .compilers_to_recognize(compilers_to_include.as_slice())
            .compilers_to_exclude(compilers_to_exclude.as_slice())
            .build()
    }

    /// Creates a new builder with default settings.
    fn new() -> Self {
        // FIXME: replace generic with gcc, when it's implemented
        Builder {
            interpreters: vec![
                // ignore executables which are not compilers,
                Box::new(IgnoreByPath::new()),
                // recognize default compiler
                Box::new(Generic::from(&[PathBuf::from("/usr/bin/g++")])),
            ],
        }
    }

    /// Factory method to create a new tool from the builder.
    fn build(self) -> impl Interpreter {
        Any::new(self.interpreters)
    }

    /// Adds new interpreters to recognize as compilers by executable name.
    fn compilers_to_recognize(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add the new compilers at the end of the interpreters.
            let tool = Generic::from(compilers);
            self.interpreters.push(Box::new(tool));
        }
        self
    }

    /// Adds new interpreters to recognize as non-compilers by executable names.
    fn compilers_to_exclude(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add these new compilers at the front of the interpreters.
            let tool = IgnoreByPath::from(compilers);
            self.interpreters.insert(0, Box::new(tool));
        }
        self
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder::new()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::{CompilerCall, Execution, Recognition};
    use super::*;
    use crate::{vec_of_pathbuf, vec_of_strings};

    #[test]
    fn test_builder() {
        let sut = Builder::new().build();

        let input = any_execution();
        match sut.recognize(&input) {
            Recognition::Success(CompilerCall { .. }) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_builder_with_compilers_to_exclude() {
        let compilers = vec_of_pathbuf!["/usr/bin/g++"];
        let sut = Builder::new().compilers_to_exclude(&compilers).build();

        let input = any_execution();
        match sut.recognize(&input) {
            Recognition::Ignored => assert!(true),
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

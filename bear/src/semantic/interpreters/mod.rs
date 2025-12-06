// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. Based on the configuration, it sets up the
//! interpreter chain to include or exclude specific compilers.

mod combinators;
pub mod compilers;
mod ignore;
mod matchers;

use super::{Command, Interpreter};
use crate::config;
use crate::intercept::Execution;
use combinators::Any;
use compilers::CompilerInterpreter;
use ignore::IgnoreByPath;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InterpreterConfigError {
    // #[error("Compiler filter configuration error: {0}")]
    // CompilerFilter(#[from] CompilerFilterConfigurationError),
    // #[error("Source filter configuration error: {0}")]
    // SourceFilter(#[from] SourceFilterConfigurationError),
}

/// Creates an interpreter to recognize the compiler calls.
///
/// Using the configuration we can define which compilers to include and exclude.
/// The interpreter chain is built as follows:
/// 1. Generic programs to exclude
/// 2. Compilers specified to exclude
/// 3. All other compilers to include
pub fn create<'a>(config: &config::Main) -> Result<impl Interpreter + 'a, InterpreterConfigError> {
    // Build the base interpreter chain
    let mut interpreters: Vec<Box<dyn Interpreter>> = vec![
        // ignore executables which are not compilers,
        Box::new(OutputLogger::new(
            IgnoreByPath::default(),
            "coreutils_to_ignore",
        )),
    ];

    let compilers_to_exclude = compilers_to_exclude(config);
    if !compilers_to_exclude.is_empty() {
        let tool = OutputLogger::new(
            IgnoreByPath::from(&compilers_to_exclude),
            "compilers_to_ignore",
        );
        interpreters.push(Box::new(tool));
    }

    // Add compiler interpreter that handles recognition and delegation
    let compiler_tool = OutputLogger::new(CompilerInterpreter::new(), "compiler_to_recognize");
    interpreters.push(Box::new(compiler_tool));

    Ok(InputLogger::new(Any::new(interpreters)))
}

fn compilers_to_exclude(config: &config::Main) -> Vec<PathBuf> {
    config
        .compilers
        .iter()
        .filter(|compiler| compiler.ignore)
        .map(|compiler| compiler.path.clone())
        .collect()
}

struct InputLogger<T: Interpreter> {
    inner: T,
}

impl<T: Interpreter> InputLogger<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Interpreter> Interpreter for InputLogger<T> {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        log::debug!("Recognizing execution: {execution:?}");
        self.inner.recognize(execution)
    }
}

struct OutputLogger<T: Interpreter> {
    inner: T,
    name: &'static str,
}

impl<T: Interpreter> OutputLogger<T> {
    pub fn new(inner: T, name: &'static str) -> Self {
        Self { inner, name }
    }
}

impl<T: Interpreter> Interpreter for OutputLogger<T> {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        let result = self.inner.recognize(execution);
        log::debug!("{:20}: {result:?}", self.name);
        result
    }
}

// #[cfg(test)]
// mod test {
//     use std::collections::HashMap;
//
//     use super::*;
//     use crate::config;
//     use crate::intercept::{execution, Execution};
//
//     #[test]
//     fn test_create_interpreter_with_default_config() {
//         let config = config::Main::default();
//
//         let interpreter = create(&config);
//
//         let result = interpreter.recognize(&EXECUTION);
//         assert!(matches!(result, Some(_)));
//     }
//
//     #[test]
//     fn test_create_interpreter_with_compilers_to_include() {
//         let config = config::Main {
//             intercept: config::Intercept::Wrapper {
//                 executables: vec!["/usr/bin/cc".into()],
//                 path: "/usr/libexec/bear".into(),
//                 directory: "/tmp".into(),
//             },
//             ..Default::default()
//         };
//
//         let interpreter = create(&config);
//
//         let result = interpreter.recognize(&EXECUTION);
//         assert!(matches!(result, Some(_)));
//     }
//
//     #[test]
//     fn test_create_interpreter_with_compilers_to_exclude() {
//         let config = config::Main {
//             output: config::Output::Clang {
//                 compilers: vec![config::Compiler {
//                     path: PathBuf::from("/usr/bin/cc"),
//                     ignore: config::IgnoreOrConsider::Always,
//                     arguments: config::Arguments::default(),
//                 }],
//                 sources: config::SourceFilter::default(),
//                 duplicates: config::DuplicateFilter::default(),
//                 format: config::Format::default(),
//             },
//             ..Default::default()
//         };
//
//         let interpreter = create(&config);
//
//         let result = interpreter.recognize(&EXECUTION);
//         assert!(matches!(result, Recognition::Ignored(_)));
//     }
//
//     static EXECUTION: std::sync::LazyLock<Execution> = std::sync::LazyLock::new(|| {
//         execution(
//             "/usr/bin/cc",
//             vec!["cc", "-c", "-Wall", "main.c"],
//             "/home/user",
//             HashMap::new(),
//         )
//     });
// }

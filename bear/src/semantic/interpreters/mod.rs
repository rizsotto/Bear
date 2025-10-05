// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. Based on the configuration, it sets up the
//! interpreter chain to include or exclude specific compilers.

use super::interpreters::combinators::Any;
use super::interpreters::filter::{
    CompilerFilterConfigurationError, Filter, FilteringInterpreter, SourceFilterConfigurationError,
};
use super::interpreters::format::{FormatConfigurationError, FormattingInterpreter, PathFormatter};
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::{Command, Interpreter};
use crate::config;
use crate::environment::program_env;
use crate::intercept::Execution;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

mod combinators;
pub mod filter;
pub mod format;
pub mod generic;
mod ignore;
mod matchers;

#[derive(Error, Debug)]
pub enum InterpreterConfigError {
    #[error("Compiler filter configuration error: {0}")]
    CompilerFilter(#[from] CompilerFilterConfigurationError),
    #[error("Source filter configuration error: {0}")]
    SourceFilter(#[from] SourceFilterConfigurationError),
    #[error("Format configuration error: {0}")]
    Format(#[from] FormatConfigurationError),
}

/// Creates an interpreter to recognize the compiler calls.
///
/// Using the configuration we can define which compilers to include and exclude.
/// The interpreter chain is built as follows:
/// 1. Basic recognition (ignore non-compilers, recognize compilers)
/// 2. Filtering (filter by compiler and source directory)
/// 3. Formatting (format paths according to configuration)
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

    let environment: HashMap<String, String> = std::env::vars().collect();
    let compilers_to_include = compilers_to_include(config, environment);
    if !compilers_to_include.is_empty() {
        let tool = OutputLogger::new(
            Generic::from(&compilers_to_include),
            "compilers_to_recognize",
        );
        interpreters.push(Box::new(tool));
    }

    let base_interpreter = Any::new(interpreters);

    // Wrap with filtering and formatting based on output configuration
    match &config.output {
        config::Output::Clang {
            compilers,
            sources,
            format,
            ..
        } => {
            let filter = Filter::try_from((compilers.as_slice(), sources))?;
            let filtering =
                FilteringInterpreter::new(OutputLogger::new(base_interpreter, "filtering"), filter);
            let path_formatter = PathFormatter::try_from(&format.paths)?;
            let formatting = FormattingInterpreter::new(
                OutputLogger::new(filtering, "formatting"),
                path_formatter,
            );
            let logger = InputLogger::new(formatting);
            Ok(logger)
        }
    }
}

fn compilers_to_exclude(config: &config::Main) -> Vec<PathBuf> {
    match &config.output {
        config::Output::Clang { compilers, .. } => compilers
            .iter()
            .filter(|compiler| compiler.ignore == config::IgnoreOrConsider::Always)
            .map(|compiler| compiler.path.clone())
            .collect(),
    }
}

fn compilers_to_include(
    config: &config::Main,
    environment: HashMap<String, String>,
) -> Vec<PathBuf> {
    let mut result = Vec::new();

    // Add wrapped executables
    if let config::Intercept::Wrapper { executables, .. } = &config.intercept {
        result.extend_from_slice(executables);
    }

    // Add configured compilers
    let config::Output::Clang { compilers, .. } = &config.output;
    compilers
        .iter()
        .filter(|compiler| compiler.ignore != config::IgnoreOrConsider::Always)
        .for_each(|compiler| result.push(compiler.path.clone()));

    // Add environment compilers
    environment.into_iter().for_each(|(key, path)| {
        if program_env(&key) {
            result.push(PathBuf::from(path));
        }
    });

    result
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

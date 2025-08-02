// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. Based on the configuration, it sets up the
//! interpreter chain to include or exclude specific compilers.

use super::interpreters::combinators::Any;
use super::interpreters::filter::{
    CompilerFilterConfigurationError, FilteringInterpreter, SourceFilterConfigurationError,
};
use super::interpreters::format::{FormatConfigurationError, FormattingInterpreter};
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::Interpreter;
use crate::config;
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
// TODO: Use the CC or CXX environment variables to detect the compiler to include.
//       Use the CC or CXX environment variables and make sure those are not excluded.
//       Make sure the environment variables are passed to the method.
// TODO: Take environment variables as input.
pub fn create<'a>(config: &config::Main) -> Result<impl Interpreter + 'a, InterpreterConfigError> {
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

    // Build the base interpreter chain
    let mut interpreters: Vec<Box<dyn Interpreter>> = vec![
        // ignore executables which are not compilers,
        Box::new(IgnoreByPath::default()),
        // recognize default compiler
        Box::new(Generic::from(&[PathBuf::from("/usr/bin/cc")])),
    ];

    if !compilers_to_include.is_empty() {
        let tool = Generic::from(&compilers_to_include);
        interpreters.push(Box::new(tool));
    }

    if !compilers_to_exclude.is_empty() {
        let tool = IgnoreByPath::from(&compilers_to_exclude);
        interpreters.insert(0, Box::new(tool));
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
            let filtering =
                FilteringInterpreter::from_config(base_interpreter, compilers, sources)?;
            let formatting = FormattingInterpreter::from_config(filtering, &format.paths)?;
            Ok(formatting)
        }
        config::Output::Semantic { .. } => {
            // For semantic output, just use pass-through formatting
            let filtering = FilteringInterpreter::pass_through(base_interpreter);
            let formatting = FormattingInterpreter::pass_through(filtering);
            Ok(formatting)
        }
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

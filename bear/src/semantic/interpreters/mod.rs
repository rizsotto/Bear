// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. Based on the configuration, it sets up the
//! interpreter chain to include or exclude specific compilers.

use super::interpreters::combinators::Any;
use super::interpreters::filter::FilteringInterpreter;
use super::interpreters::format::FormattingInterpreter;
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::Interpreter;
use crate::config;
use std::collections::HashMap;
use std::path::PathBuf;

mod combinators;
pub mod filter;
pub mod format;
pub mod generic;
mod ignore;
mod matchers;

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
pub fn create<'a>(config: &config::Main) -> impl Interpreter + 'a {
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
            // First apply filtering
            let filtered_interpreter =
                FilteringInterpreter::from_config(Box::new(base_interpreter), compilers, sources)
                    .unwrap_or_else(|_| {
                        // If filtering configuration is invalid, create a pass-through filter
                        FilteringInterpreter::new(
                            Box::new(Any::new(vec![])),
                            HashMap::new(),
                            vec![],
                        )
                    });

            // Then apply formatting
            FormattingInterpreter::from_config(Box::new(filtered_interpreter), &format.paths)
                .unwrap_or_else(|_| {
                    // If formatting configuration is invalid, use pass-through
                    FormattingInterpreter::pass_through(Box::new(Any::new(vec![])))
                })
        }
        config::Output::Semantic { .. } => {
            // For semantic output, just use pass-through formatting
            FormattingInterpreter::pass_through(Box::new(base_interpreter))
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

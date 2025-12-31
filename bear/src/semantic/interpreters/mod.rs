// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. Based on the configuration, it sets up the
//! interpreter chain to include or exclude specific compilers.

mod combinators;
pub mod compilers;
mod ignore;
mod matchers;

use super::Interpreter;
use crate::config;

use combinators::{Any, InputLogger, OutputLogger};
use compilers::CompilerInterpreter;
use ignore::IgnoreByPath;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InterpreterConfigError {
    // #[error("Compiler filter configuration error: {0}")]
    // CompilerFilter(#[from] CompilerFilterConfigurationError),
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
    let tool = CompilerInterpreter::new_with_config(&config.compilers);
    interpreters.push(Box::new(tool));

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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::config;
    use crate::intercept::Execution;
    use crate::semantic::Command;

    #[test]
    fn test_create_interpreter_with_default_config() {
        let config = config::Main::default();
        let interpreter = create(&config);

        // Test that the interpreter can be created without errors
        assert!(interpreter.is_ok());
    }

    #[test]
    fn test_create_interpreter_recognizes_compiler() {
        let config = config::Main::default();
        let interpreter = create(&config).unwrap();

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "-Wall", "main.c"],
            "/home/user",
            HashMap::new(),
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());
        match result.unwrap() {
            Command::Compiler(_) => (), // Expected
            Command::Ignored(_) => panic!("Expected compiler command, got ignored"),
        }
    }

    #[test]
    fn test_create_interpreter_ignores_coreutils() {
        let config = config::Main::default();
        let interpreter = create(&config).unwrap();

        let execution = Execution::from_strings(
            "/usr/bin/ls",
            vec!["ls", "-la"],
            "/home/user",
            HashMap::new(),
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());
        match result.unwrap() {
            Command::Ignored(_) => (), // Expected
            Command::Compiler(_) => panic!("Expected ignored command, got compiler"),
        }
    }

    #[test]
    fn test_create_interpreter_with_compilers_to_exclude() {
        let config = config::Main {
            compilers: vec![config::Compiler {
                path: PathBuf::from("/usr/bin/gcc"),
                as_: None,
                ignore: true,
            }],
            ..Default::default()
        };

        let interpreter = create(&config).unwrap();

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "main.c"],
            "/home/user",
            HashMap::new(),
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());
        match result.unwrap() {
            Command::Ignored(_) => (), // Expected - gcc should be ignored
            Command::Compiler(_) => panic!("Expected ignored command, got compiler"),
        }
    }

    #[test]
    fn test_compilers_to_exclude_function() {
        let config = config::Main {
            compilers: vec![
                config::Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    as_: None,
                    ignore: true,
                },
                config::Compiler {
                    path: PathBuf::from("/usr/bin/clang"),
                    as_: None,
                    ignore: false,
                },
            ],
            ..Default::default()
        };

        let excluded = compilers_to_exclude(&config);
        assert_eq!(excluded.len(), 1);
        assert_eq!(excluded[0], PathBuf::from("/usr/bin/gcc"));
    }

    #[test]
    fn test_windows_gcc_exe_regression() {
        // Regression test for Windows CI failure where gcc.exe was not recognized
        let config = config::Main::default();
        let interpreter = create(&config).unwrap();

        // Test with .exe extension - this simulates Windows executables
        let execution = Execution::from_strings(
            "gcc.exe",
            vec!["gcc.exe", "-fplugin=libexample.so", "-c", "test.c"],
            "/tmp",
            HashMap::new(),
        );

        // This should recognize gcc.exe as a compiler, not ignore it
        let result = interpreter.recognize(&execution);
        assert!(
            result.is_some(),
            "gcc.exe should be recognized as a compiler command"
        );

        match result.unwrap() {
            Command::Compiler(_) => {
                // This is expected - gcc.exe should be recognized as a compiler
            }
            Command::Ignored(_) => {
                panic!("gcc.exe was incorrectly ignored instead of being recognized as a compiler");
            }
        }
    }

    #[test]
    fn test_various_windows_exe_compilers() {
        let config = config::Main::default();
        let interpreter = create(&config).unwrap();

        let test_cases = vec![
            "gcc.exe",
            "g++.exe",
            "clang.exe",
            "clang++.exe",
            "gfortran.exe",
            "nvcc.exe",
        ];

        for executable_name in test_cases {
            let execution = Execution::from_strings(
                executable_name,
                vec![executable_name, "-c", "test.c"],
                "/tmp",
                HashMap::new(),
            );

            let result = interpreter.recognize(&execution);
            assert!(
                result.is_some(),
                "{} should be recognized as a compiler",
                executable_name
            );

            match result.unwrap() {
                Command::Compiler(_) => {
                    // Expected
                }
                Command::Ignored(_) => {
                    panic!("{} was incorrectly ignored", executable_name);
                }
            }
        }
    }
}

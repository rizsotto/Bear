// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. Based on the configuration, it sets up the
//! interpreter chain to include or exclude specific compilers.

mod combinators;
pub mod compilers;
mod ignore;
pub(crate) mod matchers;
pub mod resolve;

use super::Interpreter;
use crate::config;

use combinators::{Any, InputLogger, OutputLogger};
use compilers::CompilerInterpreter;
use ignore::IgnoreByPath;
use resolve::ResolveExecutable;
/// Creates an interpreter to recognize the compiler calls.
///
/// Using the configuration we can define which compilers to include and exclude.
/// The interpreter chain is built as follows:
/// 1. Generic programs to exclude
/// 2. Compilers specified to exclude
/// 3. All other compilers to include
pub fn create<'a>(config: &config::Main, confstr_path: String) -> impl Interpreter + 'a {
    // Build the base interpreter chain
    let mut interpreters: Vec<Box<dyn Interpreter>> = vec![
        // ignore executables which are not compilers,
        Box::new(OutputLogger::new(IgnoreByPath::default(), "coreutils_to_ignore")),
    ];

    let compilers_to_exclude: Vec<_> =
        config.compilers.iter().filter(|compiler| compiler.ignore).map(|compiler| &compiler.path).collect();
    if !compilers_to_exclude.is_empty() {
        let tool = OutputLogger::new(IgnoreByPath::from(compilers_to_exclude), "compilers_to_ignore");
        interpreters.push(Box::new(tool));
    }

    // Add compiler interpreter that handles recognition and delegation
    let tool = CompilerInterpreter::new_with_config(&config.compilers);
    interpreters.push(Box::new(tool));

    // Wrap the chain with executable path resolution so bare filenames
    // from preload p-variant interceptions are resolved to absolute paths.
    ResolveExecutable::new(InputLogger::new(Any::new(interpreters)), confstr_path)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;
    use crate::config;
    use crate::intercept::Execution;
    use crate::semantic::RecognizeResult;

    #[test]
    fn test_create_interpreter_with_default_config() {
        let config = config::Main::default();
        let _ = create(&config, "/usr/bin:/bin".to_string());
    }

    #[test]
    fn test_create_interpreter_recognizes_compiler() {
        let config = config::Main::default();
        let interpreter = create(&config, "/usr/bin:/bin".to_string());

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "-Wall", "main.c"],
            "/home/user",
            HashMap::new(),
        );

        assert!(matches!(interpreter.recognize(execution), RecognizeResult::Recognized(_)));
    }

    #[test]
    fn test_create_interpreter_ignores_coreutils() {
        let config = config::Main::default();
        let interpreter = create(&config, "/usr/bin:/bin".to_string());

        let execution =
            Execution::from_strings("/usr/bin/ls", vec!["ls", "-la"], "/home/user", HashMap::new());

        assert!(matches!(interpreter.recognize(execution), RecognizeResult::Ignored(_)));
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

        let interpreter = create(&config, "/usr/bin:/bin".to_string());

        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "main.c"],
            "/home/user",
            HashMap::new(),
        );

        assert!(matches!(interpreter.recognize(execution), RecognizeResult::Ignored(_)));
    }

    #[test]
    fn test_compilers_to_exclude_ignores_only_flagged() {
        let config = config::Main {
            compilers: vec![
                config::Compiler { path: PathBuf::from("/usr/bin/gcc"), as_: None, ignore: true },
                config::Compiler { path: PathBuf::from("/usr/bin/clang"), as_: None, ignore: false },
            ],
            ..Default::default()
        };

        let interpreter = create(&config, "/usr/bin:/bin".to_string());

        let gcc =
            Execution::from_strings("/usr/bin/gcc", vec!["gcc", "-c", "test.c"], "/tmp", HashMap::new());
        assert!(matches!(interpreter.recognize(gcc), RecognizeResult::Ignored(_)));

        let clang =
            Execution::from_strings("/usr/bin/clang", vec!["clang", "-c", "test.c"], "/tmp", HashMap::new());
        assert!(matches!(interpreter.recognize(clang), RecognizeResult::Recognized(_)));
    }

    #[test]
    fn test_windows_gcc_exe_regression() {
        let config = config::Main::default();
        let interpreter = create(&config, "/usr/bin:/bin".to_string());

        let execution = Execution::from_strings(
            "gcc.exe",
            vec!["gcc.exe", "-fplugin=libexample.so", "-c", "test.c"],
            "/tmp",
            HashMap::new(),
        );

        assert!(matches!(interpreter.recognize(execution), RecognizeResult::Recognized(_)));
    }

    #[test]
    fn test_various_windows_exe_compilers() {
        let config = config::Main::default();
        let interpreter = create(&config, "/usr/bin:/bin".to_string());

        let test_cases = vec!["gcc.exe", "g++.exe", "clang.exe", "clang++.exe", "gfortran.exe", "nvcc.exe"];

        for executable_name in test_cases {
            let execution = Execution::from_strings(
                executable_name,
                vec![executable_name, "-c", "test.c"],
                "/tmp",
                HashMap::new(),
            );

            assert!(
                matches!(interpreter.recognize(execution), RecognizeResult::Recognized(_)),
                "{} should be recognized",
                executable_name
            );
        }
    }
}

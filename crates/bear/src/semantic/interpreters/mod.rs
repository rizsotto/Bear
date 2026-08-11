// SPDX-License-Identifier: GPL-3.0-or-later

//! This module provides the main entry point for creating interpreters to
//! recognize compiler calls. The driver supplies the already-resolved
//! parameters, and this module sets up the interpreter chain to include or
//! exclude specific compilers.

mod combinators;
pub mod compilers;
mod ignore;
pub(crate) mod matchers;

use super::Interpreter;

use combinators::{Any, InputLogger, OutputLogger};
use compilers::CompilerInterpreter;
use compilers::compiler_recognition::CompilerHints;
use ignore::IgnoreByPath;
use std::path::PathBuf;

/// Creates an interpreter to recognize the compiler calls.
///
/// The chain composition is a semantic concern, so it stays here rather than
/// in the driver: `Any`, the loggers and `IgnoreByPath` remain private.
/// The driver decides only what goes in, in semantic terms:
///
/// * `hints` - classification overrides for known compiler paths
/// * `ignored` - paths whose invocations are dropped, matched by filename
/// * `from_response_files` - inline `@file` references before parsing
/// * `from_environment` - fold compiler environment variables into arguments
///
/// The interpreter chain is built as follows:
/// 1. Generic programs to exclude
/// 2. Compilers specified to exclude
/// 3. All other compilers to include
pub fn create<'a>(
    hints: CompilerHints,
    ignored: Vec<PathBuf>,
    from_response_files: bool,
    from_environment: bool,
) -> impl Interpreter + 'a {
    // Build the base interpreter chain
    let mut interpreters: Vec<Box<dyn Interpreter>> = vec![
        // ignore executables which are not compilers,
        Box::new(OutputLogger::new(IgnoreByPath::default(), "coreutils_to_ignore")),
    ];

    if !ignored.is_empty() {
        let tool = OutputLogger::new(IgnoreByPath::from(ignored), "compilers_to_ignore");
        interpreters.push(Box::new(tool));
    }

    // Add compiler interpreter that handles recognition and delegation
    let tool = CompilerInterpreter::new_with_format(hints, from_response_files, from_environment);
    interpreters.push(Box::new(tool));

    // The outer interpreter is logging the inputs
    InputLogger::new(Any::new(interpreters))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;
    use crate::semantic::RecognizeResult;
    use intercept::Execution;

    /// The default wiring: no hints, nothing ignored, and the default
    /// argument handling (`format.arguments`: response-file inlining off,
    /// environment-flag folding on).
    fn create_default<'a>() -> impl Interpreter + 'a {
        create(CompilerHints::new(), vec![], false, true)
    }

    #[test]
    fn test_create_interpreter_with_default_config() {
        let _ = create_default();
    }

    #[test]
    fn test_create_interpreter_recognizes_compiler() {
        let interpreter = create_default();

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
        let interpreter = create_default();

        let execution =
            Execution::from_strings("/usr/bin/ls", vec!["ls", "-la"], "/home/user", HashMap::new());

        assert!(matches!(interpreter.recognize(execution), RecognizeResult::Ignored(_)));
    }

    #[test]
    fn test_create_interpreter_with_compilers_to_exclude() {
        let interpreter = create(CompilerHints::new(), vec![PathBuf::from("/usr/bin/gcc")], false, true);

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
        // Only the ignored path reaches `create`; the other configured
        // compiler contributes a hint, which does not suppress recognition.
        let mut hints = CompilerHints::new();
        hints.add(&PathBuf::from("/usr/bin/clang"), None).unwrap();

        let interpreter = create(hints, vec![PathBuf::from("/usr/bin/gcc")], false, true);

        let gcc =
            Execution::from_strings("/usr/bin/gcc", vec!["gcc", "-c", "test.c"], "/tmp", HashMap::new());
        assert!(matches!(interpreter.recognize(gcc), RecognizeResult::Ignored(_)));

        let clang =
            Execution::from_strings("/usr/bin/clang", vec!["clang", "-c", "test.c"], "/tmp", HashMap::new());
        assert!(matches!(interpreter.recognize(clang), RecognizeResult::Recognized(_)));
    }

    #[test]
    fn test_windows_gcc_exe_regression() {
        let interpreter = create_default();

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
        let interpreter = create_default();

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

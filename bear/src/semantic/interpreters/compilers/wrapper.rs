// SPDX-License-Identifier: GPL-3.0-or-later

//! Wrapper interpreter for handling compiler wrappers like ccache, distcc, and sccache.
//!
//! This module provides support for recognizing and processing compiler wrappers that
//! act as intermediaries between build systems and actual compilers. The interpreter
//! extracts the real compiler from wrapper invocations and delegates to the main
//! CompilerInterpreter for processing the real compiler command.

use super::compiler_recognition::CompilerRecognizer;
use crate::config::CompilerType;
use crate::intercept::Execution;
use crate::semantic::{Interpreter, RecognizeResult};

use std::path::{Path, PathBuf};
use std::sync::Weak;

/// Interpreter for compiler wrappers (ccache, distcc, sccache).
///
/// This interpreter handles the complexity of extracting the real compiler from
/// wrapper invocations and delegates to the main CompilerInterpreter via weak references.
/// It supports both explicit wrapper usage (e.g., `ccache gcc`) and
/// masquerading setups where the wrapper is symlinked as the compiler name.
pub struct WrapperInterpreter {
    recognizer: Weak<CompilerRecognizer>,
    delegate: Weak<dyn Interpreter>,
}

impl WrapperInterpreter {
    /// Creates a new wrapper interpreter with weak references to the recognizer and delegate.
    pub fn new(recognizer: Weak<CompilerRecognizer>, delegate: Weak<dyn Interpreter>) -> Self {
        Self { recognizer, delegate }
    }

    /// Extracts the real compiler path and filtered arguments from wrapper invocation.
    fn extract_real_compiler(&self, wrapper_name: &str, args: &[String]) -> Option<(PathBuf, Vec<String>)> {
        match wrapper_name {
            "ccache" => self.handle_ccache(args),
            "distcc" => self.handle_distcc(args),
            "sccache" => self.handle_sccache(args),
            _ => None,
        }
    }

    /// Handles ccache wrapper invocations.
    fn handle_ccache(&self, args: &[String]) -> Option<(PathBuf, Vec<String>)> {
        let recognizer = self.recognizer.upgrade()?;

        if args.len() > 1 {
            // Explicit compiler - ccache gcc -c main.c
            let compiler_path = PathBuf::from(&args[1]);

            // Use CompilerRecognizer to validate it's actually a compiler
            if recognizer.recognize(&compiler_path).is_some() {
                return Some((compiler_path, args[1..].to_vec()));
            }
        }

        None
    }

    /// Handles sccache wrapper invocations.
    fn handle_sccache(&self, args: &[String]) -> Option<(PathBuf, Vec<String>)> {
        // sccache behavior is similar to ccache
        self.handle_ccache(args)
    }

    /// Handles distcc wrapper invocations.
    fn handle_distcc(&self, args: &[String]) -> Option<(PathBuf, Vec<String>)> {
        // distcc can have its own options before the compiler
        let compiler_index = {
            let mut index = 1;
            while index < args.len() {
                let arg = &args[index];
                let arg_count = Self::distcc_option_count(arg);
                if arg_count > 0 {
                    index += arg_count;
                } else {
                    break;
                }
            }

            index
        };

        if compiler_index < args.len() {
            let compiler_path = PathBuf::from(&args[compiler_index]);

            let recognizer = self.recognizer.upgrade()?;
            if recognizer.recognize(&compiler_path).is_some() {
                return Some((compiler_path, args[compiler_index..].to_vec()));
            }
        }

        None
    }

    /// Detects the wrapper type from the executable name.
    fn detect_wrapper_name(executable: &Path) -> Option<String> {
        let name = executable.file_stem()?.to_str()?;
        match name {
            "ccache" | "distcc" | "sccache" => Some(name.to_string()),
            _ => None,
        }
    }

    /// Checks if an argument is a distcc-specific option.
    fn distcc_option_count(arg: &str) -> usize {
        match arg {
            "-j" | "--jobs" => 2,
            "-v" | "--verbose" | "-i" | "--show-hosts" | "--scan-avail" | "--show-principal" => 1,
            _ => 0,
        }
    }
}

impl Interpreter for WrapperInterpreter {
    fn recognize(&self, execution: Execution) -> RecognizeResult {
        let Some(wrapper_name) = Self::detect_wrapper_name(&execution.executable) else {
            return RecognizeResult::NotRecognized(execution);
        };

        let Some((real_compiler_path, filtered_args)) =
            self.extract_real_compiler(&wrapper_name, &execution.arguments)
        else {
            return RecognizeResult::NotRecognized(execution);
        };

        let Some(recognizer) = self.recognizer.upgrade() else {
            return RecognizeResult::NotRecognized(execution);
        };
        let Some(compiler_type) = recognizer.recognize(&real_compiler_path) else {
            return RecognizeResult::NotRecognized(execution);
        };
        if matches!(compiler_type, CompilerType::Wrapper) {
            return RecognizeResult::NotRecognized(execution);
        }

        let Some(delegate) = self.delegate.upgrade() else {
            return RecognizeResult::NotRecognized(execution);
        };

        // Move working_dir and environment from original execution
        let real_execution = Execution {
            executable: real_compiler_path,
            arguments: filtered_args,
            working_dir: execution.working_dir,
            environment: execution.environment,
        };

        delegate.recognize(real_execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{CompilerCommand, MockInterpreter, RecognizeResult};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn create_execution(args: Vec<&str>) -> Execution {
        Execution::from_strings(args[0], args, "/project", HashMap::new())
    }

    fn create_sut(
        mock: impl Interpreter + 'static,
    ) -> (impl Interpreter, (Arc<CompilerRecognizer>, Arc<impl Interpreter>)) {
        let recognizer = Arc::new(CompilerRecognizer::new());
        let delegate = Arc::new(mock);

        let sut = WrapperInterpreter::new(
            Arc::downgrade(&recognizer),
            Arc::downgrade(&delegate) as Weak<dyn Interpreter>,
        );

        (sut, (recognizer, delegate))
    }

    #[test]
    fn test_detect_wrapper_name() {
        let sut = |path_str| {
            let path = PathBuf::from(path_str);
            WrapperInterpreter::detect_wrapper_name(&path)
        };

        assert_eq!(sut("/usr/bin/ccache"), Some("ccache".to_string()));
        assert_eq!(sut("/opt/distcc"), Some("distcc".to_string()));
        assert_eq!(sut("sccache"), Some("sccache".to_string()));
        assert_eq!(sut("/usr/bin/gcc"), None);
        assert_eq!(sut("make"), None);
    }

    #[test]
    fn test_is_distcc_option() {
        let sut = |arg| WrapperInterpreter::distcc_option_count(arg);

        assert_eq!(2, sut("-j"));
        assert_eq!(2, sut("--jobs"));
        assert_eq!(1, sut("-v"));
        assert_eq!(1, sut("--verbose"));
        assert_eq!(1, sut("-i"));
        assert_eq!(1, sut("--show-hosts"));
        assert_eq!(1, sut("--scan-avail"));
        assert_eq!(1, sut("--show-principal"));
        assert_eq!(0, sut("-c"));
        assert_eq!(0, sut("-Wall"));
        assert_eq!(0, sut("--output"));
    }

    #[test]
    fn test_recognize_valid_wrapper_calls() {
        let executions = vec![
            (create_execution(vec!["ccache", "gcc", "-c", "main.c"]), "gcc"),
            (create_execution(vec!["/usr/bin/ccache", "gcc", "-c", "main.c"]), "gcc"),
            (create_execution(vec!["ccache", "/usr/bin/gcc", "-c", "main.c"]), "/usr/bin/gcc"),
            (create_execution(vec!["ccache", "clang", "-c", "main.c"]), "clang"),
            (create_execution(vec!["ccache", "/usr/bin/clang", "-c", "main.c"]), "/usr/bin/clang"),
            (create_execution(vec!["sccache", "gcc", "-c", "main.c"]), "gcc"),
            (create_execution(vec!["sccache", "clang", "-c", "main.c"]), "clang"),
            (create_execution(vec!["distcc", "-j", "4", "gcc", "-c", "main.c"]), "gcc"),
            (create_execution(vec!["distcc", "clang", "-c", "main.c"]), "clang"),
        ];
        let mock = {
            let mut mock = MockInterpreter::new();
            mock.expect_recognize().returning(|execution| {
                RecognizeResult::Recognized(CompilerCommand::new(
                    execution.working_dir,
                    execution.executable,
                    vec![],
                ))
            });

            mock
        };

        let (sut, _context) = create_sut(mock);

        for (execution, compiler) in executions {
            let result = sut.recognize(execution);

            let RecognizeResult::Recognized(cmd) = result else {
                panic!("wrapper call should be recognized");
            };
            assert_eq!(cmd.executable, PathBuf::from(compiler));
        }
    }

    #[test]
    fn test_recognize_fails_non_wrapper_calls() {
        let executions = vec![
            create_execution(vec!["gcc", "-c", "main.c"]),
            create_execution(vec!["make", "all"]),
            create_execution(vec!["ccache"]),
            create_execution(vec!["ccache", "make", "all"]),
            create_execution(vec!["ccache", "distcc", "gcc", "-c", "main.c"]),
        ];
        let mock = {
            let mut mock = MockInterpreter::new();
            mock.expect_recognize().returning(|execution| {
                RecognizeResult::Recognized(CompilerCommand::new(
                    execution.working_dir,
                    execution.executable,
                    vec![],
                ))
            });

            mock
        };

        let (sut, _context) = create_sut(mock);

        for execution in executions {
            let result = sut.recognize(execution);

            assert!(matches!(result, RecognizeResult::NotRecognized(_)), "call should not be recognized");
        }
    }

    #[test]
    fn test_recognize_preserves_working_dir_and_environment() {
        let environment = {
            let mut builder = HashMap::new();
            builder.insert("CC", "gcc");
            builder
        };
        let execution = Execution::from_strings(
            "/usr/bin/ccache",
            vec!["ccache", "gcc", "-c", "main.c"],
            "/custom/dir",
            environment,
        );

        let mock = {
            let mut mock = MockInterpreter::new();
            mock.expect_recognize()
                .withf(|execution| {
                    *execution.working_dir == *"/custom/dir"
                        && execution.environment.get("CC") == Some(&"gcc".to_string())
                })
                .returning(|execution| {
                    RecognizeResult::Recognized(CompilerCommand::new(
                        execution.working_dir,
                        execution.executable,
                        vec![],
                    ))
                });
            mock
        };

        let (sut, _context) = create_sut(mock);
        let result = sut.recognize(execution);

        assert!(
            matches!(result, RecognizeResult::Recognized(_)),
            "Should delegate successfully and preserve execution context"
        );
    }

    #[test]
    fn test_recognize_filters_wrapper_args_from_delegated_execution() {
        let execution = Execution::from_strings(
            "/usr/bin/distcc",
            vec!["distcc", "-j", "4", "gcc", "-c", "main.c", "-o", "main.o"],
            "/project",
            HashMap::new(),
        );

        let mock = {
            let mut mock = MockInterpreter::new();
            mock.expect_recognize()
                .withf(|execution| {
                    *execution.executable == *"gcc"
                        && execution.arguments == vec!["gcc", "-c", "main.c", "-o", "main.o"]
                })
                .returning(|execution| {
                    RecognizeResult::Recognized(CompilerCommand::new(
                        execution.working_dir,
                        execution.executable,
                        vec![],
                    ))
                });
            mock
        };

        let (sut, _context) = create_sut(mock);
        let result = sut.recognize(execution);

        assert!(
            matches!(result, RecognizeResult::Recognized(_)),
            "Wrapper should strip its own args before delegating"
        );
    }

    #[test]
    fn test_recognize_returns_none_when_delegate_rejects() {
        let execution = create_execution(vec!["ccache", "gcc", "-c", "main.c"]);

        let mock = {
            let mut delegate = MockInterpreter::new();
            delegate.expect_recognize().returning(RecognizeResult::NotRecognized);

            delegate
        };

        let (sut, _context) = create_sut(mock);
        let result = sut.recognize(execution);

        assert!(
            matches!(result, RecognizeResult::NotRecognized(_)),
            "Should return NotRecognized when delegate does not recognize the compiler"
        );
    }
}

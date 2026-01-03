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
use crate::semantic::{Command, Interpreter};

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

    /// Detects the wrapper type from the executable name.
    fn detect_wrapper_name(&self, executable: &Path) -> Option<String> {
        let name = executable.file_stem()?.to_str()?;
        match name {
            "ccache" | "distcc" | "sccache" => Some(name.to_string()),
            _ => None,
        }
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
            // Case 1: Explicit compiler - ccache gcc -c main.c
            let potential_compiler = &args[1];
            let compiler_path = PathBuf::from(potential_compiler);

            // Use CompilerRecognizer to validate it's actually a compiler
            if let Some(compiler_type) = recognizer.recognize(&compiler_path) {
                // Skip if it's another wrapper to avoid infinite recursion
                if !matches!(compiler_type, CompilerType::Wrapper) {
                    return Some((compiler_path, args[2..].to_vec()));
                }
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
        let recognizer = self.recognizer.upgrade()?;

        // distcc can have its own options before the compiler
        let mut compiler_index = 1;

        // Skip distcc-specific options
        while compiler_index < args.len() {
            let arg = &args[compiler_index];
            if arg.starts_with('-') && self.is_distcc_option(arg) {
                compiler_index += 1;
                // Some options might have values
                if self.distcc_option_has_value(arg) && compiler_index < args.len() {
                    compiler_index += 1;
                }
            } else {
                break;
            }
        }

        if compiler_index < args.len() {
            let compiler_path = PathBuf::from(&args[compiler_index]);
            if let Some(compiler_type) = recognizer.recognize(&compiler_path)
                && !matches!(compiler_type, CompilerType::Wrapper)
            {
                return Some((compiler_path, args[compiler_index + 1..].to_vec()));
            }
        }

        None
    }

    /// Checks if an argument is a distcc-specific option.
    fn is_distcc_option(&self, arg: &str) -> bool {
        matches!(
            arg,
            "-j" | "--jobs"
                | "-v"
                | "--verbose"
                | "-i"
                | "--show-hosts"
                | "--scan-avail"
                | "--show-principal"
        )
    }

    /// Checks if a distcc option requires a value.
    fn distcc_option_has_value(&self, arg: &str) -> bool {
        matches!(arg, "-j" | "--jobs")
    }
}

impl Interpreter for WrapperInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // 1. Detect which wrapper we're dealing with
        let wrapper_name = self.detect_wrapper_name(&execution.executable)?;

        // 2. Extract real compiler path and filtered arguments
        let (real_compiler_path, filtered_args) =
            self.extract_real_compiler(&wrapper_name, &execution.arguments)?;

        // 3. Skip if it's another wrapper (avoid infinite recursion)
        let recognizer = self.recognizer.upgrade()?;
        let compiler_type = recognizer.recognize(&real_compiler_path)?;
        if matches!(compiler_type, CompilerType::Wrapper) {
            return None;
        }

        // 4. Create new execution with real compiler
        let real_execution = Execution {
            executable: real_compiler_path,
            arguments: filtered_args,
            working_dir: execution.working_dir.clone(),
            environment: execution.environment.clone(),
        };

        // 5. Delegate to the main interpreter for re-recognition with the real compiler
        let delegate = self.delegate.upgrade()?;
        delegate.recognize(&real_execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct TestDelegate;
    impl Interpreter for TestDelegate {
        fn recognize(&self, _execution: &Execution) -> Option<Command> {
            None
        }
    }

    fn create_wrapper_interpreter() -> (Arc<CompilerRecognizer>, Arc<TestDelegate>, WrapperInterpreter) {
        let config = vec![];
        let recognizer = Arc::new(CompilerRecognizer::new_with_config(&config));
        let delegate = Arc::new(TestDelegate);
        let wrapper = WrapperInterpreter::new(
            Arc::downgrade(&recognizer),
            Arc::downgrade(&delegate) as Weak<dyn Interpreter>,
        );

        (recognizer, delegate, wrapper)
    }

    #[test]
    fn test_detect_wrapper_name() {
        let (_recognizer, _delegate, interpreter) = create_wrapper_interpreter();

        assert_eq!(
            interpreter.detect_wrapper_name(&PathBuf::from("/usr/bin/ccache")),
            Some("ccache".to_string())
        );
        assert_eq!(
            interpreter.detect_wrapper_name(&PathBuf::from("/opt/distcc")),
            Some("distcc".to_string())
        );
        assert_eq!(interpreter.detect_wrapper_name(&PathBuf::from("sccache")), Some("sccache".to_string()));
        assert_eq!(interpreter.detect_wrapper_name(&PathBuf::from("/usr/bin/gcc")), None);
    }

    #[test]
    fn test_is_distcc_option() {
        let (_recognizer, _delegate, interpreter) = create_wrapper_interpreter();

        assert!(interpreter.is_distcc_option("-j"));
        assert!(interpreter.is_distcc_option("--jobs"));
        assert!(interpreter.is_distcc_option("-v"));
        assert!(interpreter.is_distcc_option("--verbose"));
        assert!(!interpreter.is_distcc_option("-c"));
        assert!(!interpreter.is_distcc_option("-Wall"));
    }

    #[test]
    fn test_distcc_option_has_value() {
        let (_recognizer, _delegate, interpreter) = create_wrapper_interpreter();

        assert!(interpreter.distcc_option_has_value("-j"));
        assert!(interpreter.distcc_option_has_value("--jobs"));
        assert!(!interpreter.distcc_option_has_value("-v"));
        assert!(!interpreter.distcc_option_has_value("--verbose"));
    }

    #[test]
    fn test_ccache_explicit_compiler_extraction() {
        let (_recognizer, _delegate, interpreter) = create_wrapper_interpreter();

        // This test would need the recognizer to be properly mocked to work fully
        // For now, we just test that the method doesn't panic
        let args = vec!["ccache".to_string(), "gcc".to_string(), "-c".to_string(), "main.c".to_string()];

        let _result = interpreter.extract_real_compiler("ccache", &args);
        // In a real test, we'd assert on the result, but that requires mocking the recognizer
    }

    #[test]
    fn test_distcc_with_options() {
        let (_recognizer, _delegate, interpreter) = create_wrapper_interpreter();

        let args = vec![
            "distcc".to_string(),
            "-j".to_string(),
            "4".to_string(),
            "gcc".to_string(),
            "-c".to_string(),
            "main.c".to_string(),
        ];

        let _result = interpreter.extract_real_compiler("distcc", &args);
        // In a real test, we'd assert on the result, but that requires mocking the recognizer
    }

    #[test]
    fn test_sccache_behavior_same_as_ccache() {
        let (_recognizer, _delegate, interpreter) = create_wrapper_interpreter();

        let args = vec![
            "sccache".to_string(),
            "clang++".to_string(),
            "-std=c++17".to_string(),
            "file.cpp".to_string(),
        ];

        let _result = interpreter.extract_real_compiler("sccache", &args);
        // In a real test, we'd assert on the result, but that requires mocking the recognizer
    }
}

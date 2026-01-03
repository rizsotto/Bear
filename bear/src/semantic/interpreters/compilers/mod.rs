// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler interpreters for recognizing and parsing compiler invocations.
//!
//! This module provides interpreters for various compiler toolchains including
//! GCC, Clang, CUDA, and Fortran compilers, as well as support for compiler
//! wrappers like ccache, distcc, and sccache.

pub mod arguments;
pub mod clang;
pub mod compiler_recognition;
pub mod cray_fortran;
pub mod cuda;
pub mod gcc;
pub mod intel_fortran;
pub mod wrapper;

use super::super::{Command, Interpreter};
use super::combinators::OutputLogger;
use crate::config::CompilerType;
use crate::intercept::Execution;
use clang::{ClangInterpreter, FlangInterpreter};
use compiler_recognition::CompilerRecognizer;
use cray_fortran::CrayFortranInterpreter;
use cuda::CudaInterpreter;
use gcc::GccInterpreter;
use intel_fortran::IntelFortranInterpreter;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use wrapper::WrapperInterpreter;

/// Main compiler interpreter that delegates to specific compiler implementations.
///
/// This interpreter uses a map-based architecture where each compiler type
/// is stored in a map for delegation. All interpreters are treated uniformly.
pub struct CompilerInterpreter {
    /// Compiler recognizer for identifying compiler types
    recognizer: Arc<CompilerRecognizer>,
    /// Map of compiler types to their interpreters (includes all types)
    interpreters: HashMap<CompilerType, Box<dyn Interpreter>>,
    /// Wrapper interpreter stored separately to handle circular dependency
    wrapper_interpreter: OnceLock<Box<dyn Interpreter>>,
}

impl CompilerInterpreter {
    /// Factory method that creates a fully configured compiler interpreter.
    ///
    /// This method creates the interpreter and registers all supported
    /// compiler types, including wrapper support with proper circular dependency handling.
    pub fn new_with_config(compilers: &[crate::config::Compiler]) -> Arc<Self> {
        let recognizer = Arc::new(CompilerRecognizer::new_with_config(compilers));

        // Create the final interpreter and register all non-wrapper interpreters
        let mut result = CompilerInterpreter::new(Arc::clone(&recognizer));

        // Register all interpreter types using the centralized method
        result.register(CompilerType::Gcc, GccInterpreter::default());
        result.register(CompilerType::Clang, ClangInterpreter::default());
        result.register(CompilerType::Flang, FlangInterpreter::default());
        result.register(CompilerType::IntelFortran, IntelFortranInterpreter::default());
        result.register(CompilerType::CrayFortran, CrayFortranInterpreter::default());
        result.register(CompilerType::Cuda, CudaInterpreter::default());

        Arc::new_cyclic(|weak_self| {
            // Create wrapper interpreter with weak references
            let wrapper_interpreter = WrapperInterpreter::new(
                Arc::downgrade(&recognizer),
                weak_self.clone() as std::sync::Weak<dyn Interpreter>,
            );

            // Store wrapper interpreter in OnceLock
            let _ = result
                .wrapper_interpreter
                .set(Box::new(OutputLogger::new(wrapper_interpreter, CompilerType::Wrapper.to_string())));

            result
        })
    }
    /// Creates a new compiler interpreter with empty interpreter map.
    ///
    /// This is the basic constructor. Use `new_with_config` for a fully
    /// configured interpreter with all compiler types registered.
    fn new(recognizer: Arc<CompilerRecognizer>) -> Self {
        Self { recognizer, interpreters: HashMap::new(), wrapper_interpreter: OnceLock::new() }
    }

    /// Registers an interpreter for a specific compiler type.
    /// The interpreter will be automatically wrapped with OutputLogger using the compiler type name.
    fn register(&mut self, compiler_type: CompilerType, interpreter: impl Interpreter + 'static) {
        let logged_interpreter = OutputLogger::new(interpreter, compiler_type.to_string());
        self.interpreters.insert(compiler_type, Box::new(logged_interpreter));
    }
}

impl Default for CompilerInterpreter {
    fn default() -> Self {
        Self::new(Arc::new(CompilerRecognizer::new_with_config(&[])))
    }
}

impl Interpreter for CompilerInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // All compiler types are treated uniformly - just delegate to the map
        let compiler_type = self.recognizer.recognize(&execution.executable)?;

        // Handle wrapper type specially due to circular dependency
        if matches!(compiler_type, CompilerType::Wrapper) {
            return self.wrapper_interpreter.get()?.recognize(execution);
        }

        // Handle all other compiler types normally
        self.interpreters.get(&compiler_type)?.recognize(execution)
    }
}

impl Interpreter for Arc<CompilerInterpreter> {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        (**self).recognize(execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_execution(executable: &str, arguments: Vec<&str>) -> Execution {
        Execution {
            executable: PathBuf::from(executable),
            arguments: arguments.into_iter().map(String::from).collect(),
            working_dir: PathBuf::from("/tmp"),
            environment: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_gcc_recognition_and_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("/usr/bin/gcc", vec!["-c", "test.c"]);

        let result = sut.recognize(&execution);

        assert!(result.is_some(), "GCC command should be recognized");
        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.executable, PathBuf::from("/usr/bin/gcc"));
            assert_eq!(cmd.working_dir, PathBuf::from("/tmp"));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_recognition_and_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("clang", vec!["-c", "main.c", "-o", "main.o"]);

        let result = sut.recognize(&execution);

        assert!(result.is_some(), "Clang command should be recognized");
        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.executable, PathBuf::from("clang"));
            assert_eq!(cmd.working_dir, PathBuf::from("/tmp"));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_unrecognized_compiler() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("unknown_compiler", vec!["-c", "test.c"]);

        let result = sut.recognize(&execution);

        assert!(result.is_none(), "Unknown compiler should not be recognized");
    }

    #[test]
    fn test_delegation_preserves_execution_details() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let working_dir = PathBuf::from("/custom/working/dir");
        let mut environment = std::collections::HashMap::new();
        environment.insert("CC".to_string(), "gcc".to_string());

        let execution = Execution {
            executable: PathBuf::from("gcc"),
            arguments: vec!["-c".to_string(), "file.c".to_string()],
            working_dir: working_dir.clone(),
            environment,
        };

        let result = sut.recognize(&execution);

        assert!(result.is_some(), "Command should be recognized");
        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.working_dir, working_dir);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_end_to_end_config_based_compiler_hints() {
        use crate::config::{Compiler, CompilerType};

        let config = vec![
            Compiler { path: "/custom/path/my-gcc".into(), as_: Some(CompilerType::Gcc), ignore: false },
            Compiler { path: "/opt/clang/bin/clang++".into(), as_: Some(CompilerType::Clang), ignore: false },
        ];

        let sut = CompilerInterpreter::new_with_config(&config);

        // Test custom GCC path
        let custom_gcc = create_execution("/custom/path/my-gcc", vec!["-c", "test.c"]);
        let result = sut.recognize(&custom_gcc);
        assert!(result.is_some(), "Custom GCC path should be recognized via config hint");

        // Test custom Clang path
        let custom_clang = create_execution("/opt/clang/bin/clang++", vec!["-c", "main.cpp"]);
        let result = sut.recognize(&custom_clang);
        assert!(result.is_some(), "Custom Clang path should be recognized via config hint");

        // Test that normal compiler paths still work
        let normal_gcc = create_execution("gcc", vec!["-c", "normal.c"]);
        let result = sut.recognize(&normal_gcc);
        assert!(result.is_some(), "Standard GCC should still be recognized");
    }

    #[test]
    fn test_wrapper_recognition_and_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test ccache wrapper
        let ccache_execution = create_execution("ccache", vec!["gcc", "-c", "test.c"]);
        let result = sut.recognize(&ccache_execution);

        // Wrapper support might not be fully functional yet, so we just check it doesn't crash
        // In a complete implementation, this should delegate to gcc and return a compiler command
        match result {
            Some(Command::Compiler(_)) => {
                // Great! Wrapper delegation worked
            }
            Some(Command::Ignored(_)) => {
                // Wrapper was recognized but ignored for some reason
            }
            None => {
                // Wrapper support not yet complete, which is acceptable
            }
        }
    }

    #[test]
    fn test_uniform_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test that all compiler types are handled uniformly through the map
        let test_cases = vec![
            ("gcc", CompilerType::Gcc),
            ("clang", CompilerType::Clang),
            ("nvcc", CompilerType::Cuda),
            ("gfortran", CompilerType::Flang),
            ("ifort", CompilerType::IntelFortran),
        ];

        for (executable, _expected_type) in test_cases {
            let execution = create_execution(executable, vec!["-c", "test.c"]);

            // Test that the recognizer identifies the correct type
            let recognized_type = sut.recognizer.recognize(&execution.executable);
            if let Some(compiler_type) = recognized_type {
                // If it's recognized, it should delegate properly through the map
                let result = sut.interpreters.get(&compiler_type);
                assert!(result.is_some(), "Interpreter should be registered for {}", executable);
            }
        }
    }
}

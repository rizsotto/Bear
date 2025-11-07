// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler interpreter that recognizes compiler types and delegates to specific interpreters.
//!
//! This module provides a unified entry point for compiler recognition that separates
//! the concern of identifying compiler types from the concern of parsing their arguments.

pub mod arguments;
pub mod clang;
pub mod compiler_recognition;
pub mod gcc;

use crate::intercept::Execution;
use crate::semantic::{Command, Interpreter};
use clang::ClangInterpreter;
use compiler_recognition::{CompilerRecognizer, CompilerType};
use gcc::GccInterpreter;

/// A meta-interpreter that recognizes compiler types and delegates parsing to specific interpreters.
///
/// This interpreter follows the separation of concerns principle:
/// - It handles compiler recognition (identifying what type of compiler is being invoked)
/// - It delegates argument parsing to specialized interpreters (GccInterpreter, ClangInterpreter, etc.)
///
/// The specialized interpreters no longer need to check compiler names - they focus purely
/// on parsing command-line arguments according to their specific compiler's syntax.
pub struct CompilerInterpreter {
    /// Unified compiler recognizer for identifying compiler types
    recognizer: CompilerRecognizer,
    /// GCC-specific argument parser
    gcc_interpreter: GccInterpreter,
    /// Clang-specific argument parser
    clang_interpreter: ClangInterpreter,
}

impl CompilerInterpreter {
    /// Creates a new compiler interpreter with default settings.
    pub fn new() -> Self {
        Self {
            recognizer: CompilerRecognizer::new(),
            gcc_interpreter: GccInterpreter::new(),
            clang_interpreter: ClangInterpreter::new(),
        }
    }

    /// Creates a compiler interpreter with a custom recognizer.
    pub fn with_recognizer(recognizer: CompilerRecognizer) -> Self {
        Self {
            recognizer,
            gcc_interpreter: GccInterpreter::new(),
            clang_interpreter: ClangInterpreter::new(),
        }
    }

    /// Recognizes the compiler type and delegates to the appropriate interpreter.
    fn delegate_to_interpreter(&self, execution: &Execution) -> Option<Command> {
        match self.recognizer.recognize(&execution.executable) {
            Some(CompilerType::Gcc) => {
                // Delegate to GCC interpreter for argument parsing
                self.gcc_interpreter.recognize(execution)
            }
            Some(CompilerType::Clang) => {
                // Delegate to Clang interpreter for argument parsing
                self.clang_interpreter.recognize(execution)
            }
            Some(CompilerType::Fortran) => {
                // For now, treat Fortran compilers like GCC (they often are GCC-based)
                self.gcc_interpreter.recognize(execution)
            }
            Some(CompilerType::IntelFortran) => {
                // Intel Fortran often has GCC-compatible syntax
                self.gcc_interpreter.recognize(execution)
            }
            Some(CompilerType::CrayFortran) => {
                // Cray Fortran often has GCC-compatible syntax
                self.gcc_interpreter.recognize(execution)
            }
            None => {
                // Compiler not recognized - no parsing performed
                None
            }
        }
    }
}

impl Default for CompilerInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter for CompilerInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        self.delegate_to_interpreter(execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_execution(executable: &str, args: Vec<&str>, working_dir: &str) -> Execution {
        Execution {
            executable: PathBuf::from(executable),
            arguments: args.into_iter().map(String::from).collect(),
            working_dir: PathBuf::from(working_dir),
            environment: HashMap::new(),
        }
    }

    #[test]
    fn test_gcc_recognition_and_delegation() {
        let interpreter = CompilerInterpreter::new();

        // Test various GCC executable names
        let gcc_executables = vec![
            "gcc",
            "g++",
            "cc",
            "c++",
            "/usr/bin/gcc",
            "arm-linux-gnueabi-gcc",
            "gcc-11",
        ];

        for executable in gcc_executables {
            let exec = create_execution(executable, vec![executable, "-c", "main.c"], "/project");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_some(),
                "Failed to recognize GCC executable: {}",
                executable
            );

            if let Some(Command::Compiler(cmd)) = result {
                assert_eq!(cmd.executable, PathBuf::from(executable));
                assert_eq!(cmd.working_dir, PathBuf::from("/project"));
            } else {
                panic!("Expected compiler command for: {}", executable);
            }
        }
    }

    #[test]
    fn test_clang_recognition_and_delegation() {
        let interpreter = CompilerInterpreter::new();

        // Test various Clang executable names
        let clang_executables = vec![
            "clang",
            "clang++",
            "/usr/bin/clang",
            "aarch64-linux-gnu-clang",
            "clang-15",
        ];

        for executable in clang_executables {
            let exec = create_execution(executable, vec![executable, "-c", "main.c"], "/project");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_some(),
                "Failed to recognize Clang executable: {}",
                executable
            );

            if let Some(Command::Compiler(cmd)) = result {
                assert_eq!(cmd.executable, PathBuf::from(executable));
                assert_eq!(cmd.working_dir, PathBuf::from("/project"));
            } else {
                panic!("Expected compiler command for: {}", executable);
            }
        }
    }

    #[test]
    fn test_fortran_recognition_and_delegation() {
        let interpreter = CompilerInterpreter::new();

        // Test various Fortran executable names
        let fortran_executables = vec![
            "gfortran",
            "f77",
            "f90",
            "f95",
            "/usr/bin/gfortran",
            "arm-linux-gnueabi-gfortran",
        ];

        for executable in fortran_executables {
            let exec = create_execution(executable, vec![executable, "-c", "main.f90"], "/project");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_some(),
                "Failed to recognize Fortran executable: {}",
                executable
            );

            if let Some(Command::Compiler(cmd)) = result {
                assert_eq!(cmd.executable, PathBuf::from(executable));
                assert_eq!(cmd.working_dir, PathBuf::from("/project"));
            } else {
                panic!("Expected compiler command for: {}", executable);
            }
        }
    }

    #[test]
    fn test_intel_fortran_recognition() {
        let interpreter = CompilerInterpreter::new();

        let intel_executables = vec!["ifort", "ifx"];

        for executable in intel_executables {
            let exec = create_execution(executable, vec![executable, "-c", "main.f90"], "/project");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_some(),
                "Failed to recognize Intel Fortran executable: {}",
                executable
            );
        }
    }

    #[test]
    fn test_cray_fortran_recognition() {
        let interpreter = CompilerInterpreter::new();

        let cray_executables = vec!["crayftn", "ftn"];

        for executable in cray_executables {
            let exec = create_execution(executable, vec![executable, "-c", "main.f90"], "/project");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_some(),
                "Failed to recognize Cray Fortran executable: {}",
                executable
            );
        }
    }

    #[test]
    fn test_unrecognized_compiler() {
        let interpreter = CompilerInterpreter::new();

        let unknown_executables = vec!["rustc", "javac", "make", "cmake", "unknown-compiler"];

        for executable in unknown_executables {
            let exec = create_execution(executable, vec![executable, "input.file"], "/project");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_none(),
                "Should not recognize unknown executable: {}",
                executable
            );
        }
    }

    #[test]
    fn test_delegation_preserves_execution_details() {
        let interpreter = CompilerInterpreter::new();

        let exec = create_execution(
            "/custom/path/gcc-11",
            vec![
                "gcc-11",
                "-Wall",
                "-O2",
                "-c",
                "complex.c",
                "-o",
                "complex.o",
            ],
            "/work/project",
        );

        let result = interpreter.recognize(&exec);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            // Verify execution details are preserved through delegation
            assert_eq!(cmd.executable, PathBuf::from("/custom/path/gcc-11"));
            assert_eq!(cmd.working_dir, PathBuf::from("/work/project"));

            // Verify arguments were parsed (should have multiple argument groups)
            assert!(
                cmd.arguments.len() > 1,
                "Arguments should be parsed into groups"
            );
        }
    }

    #[test]
    fn test_path_independence() {
        let interpreter = CompilerInterpreter::new();

        // Same compiler name with different paths should be recognized identically
        let paths = vec![
            "gcc",
            "./gcc",
            "/usr/bin/gcc",
            "/opt/gcc/bin/gcc",
            "/custom/weird/path/gcc",
        ];

        for path in paths {
            let exec = create_execution(path, vec!["gcc", "-c", "test.c"], "/tmp");
            let result = interpreter.recognize(&exec);

            assert!(
                result.is_some(),
                "Failed to recognize gcc at path: {}",
                path
            );
        }
    }

    #[test]
    fn test_compiler_type_delegation_separation() {
        let interpreter = CompilerInterpreter::new();

        // Test that GCC and Clang are handled by different interpreters
        // This is more of a design verification than functional test

        let gcc_exec = create_execution("gcc", vec!["gcc", "-c", "test.c"], "/project");
        let clang_exec = create_execution("clang", vec!["clang", "-c", "test.c"], "/project");

        let gcc_result = interpreter.recognize(&gcc_exec);
        let clang_result = interpreter.recognize(&clang_exec);

        // Both should succeed but may have different argument parsing behavior
        assert!(gcc_result.is_some(), "GCC should be recognized and parsed");
        assert!(
            clang_result.is_some(),
            "Clang should be recognized and parsed"
        );

        // The actual parsing differences would be tested in the specific interpreter tests
    }
}

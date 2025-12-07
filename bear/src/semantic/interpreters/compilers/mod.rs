// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler interpreter that recognizes compiler types and delegates to specific interpreters.
//!
//! This module provides a unified entry point for compiler recognition that separates
//! the concern of identifying compiler types from the concern of parsing their arguments.

pub mod arguments;
pub mod clang;
pub mod compiler_recognition;
pub mod gcc;

use super::combinators::OutputLogger;
use crate::config::CompilerType;
use crate::intercept::Execution;
use crate::semantic::{Command, Interpreter};
use clang::ClangInterpreter;
use compiler_recognition::CompilerRecognizer;
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
    /// GCC-specific argument parser with logging
    gcc_interpreter: OutputLogger<GccInterpreter>,
    /// Clang-specific argument parser with logging
    clang_interpreter: OutputLogger<ClangInterpreter>,
}

impl CompilerInterpreter {
    /// Creates a new compiler interpreter with configuration-based compiler hints.
    pub fn new_with_config(compilers: &[crate::config::Compiler]) -> Self {
        Self {
            recognizer: CompilerRecognizer::new_with_config(compilers),
            ..Default::default()
        }
    }
}

impl Default for CompilerInterpreter {
    fn default() -> Self {
        Self {
            recognizer: CompilerRecognizer::default(),
            gcc_interpreter: OutputLogger::new(GccInterpreter::default(), "gcc_compiler"),
            clang_interpreter: OutputLogger::new(ClangInterpreter::default(), "clang_compiler"),
        }
    }
}

impl Interpreter for CompilerInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        {
            let this = &self;
            match this.recognizer.recognize(&execution.executable) {
                Some(CompilerType::Gcc) => {
                    // Delegate to GCC interpreter for argument parsing
                    this.gcc_interpreter.recognize(execution)
                }
                Some(CompilerType::Clang) => {
                    // Delegate to Clang interpreter for argument parsing
                    this.clang_interpreter.recognize(execution)
                }
                Some(CompilerType::Fortran) => {
                    // For now, treat Fortran compilers like GCC (they often are GCC-based)
                    this.gcc_interpreter.recognize(execution)
                }
                Some(CompilerType::IntelFortran) => {
                    // Intel Fortran often has GCC-compatible syntax
                    this.gcc_interpreter.recognize(execution)
                }
                Some(CompilerType::CrayFortran) => {
                    // Cray Fortran often has GCC-compatible syntax
                    this.gcc_interpreter.recognize(execution)
                }
                None => {
                    // Compiler not recognized - no parsing performed
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Compiler, CompilerType};
    use crate::semantic::interpreters::compilers::compiler_recognition::CompilerRecognizer;
    use crate::semantic::Interpreter;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
        let interpreter = CompilerInterpreter::default();

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
    fn test_gcc_internal_executable_end_to_end() {
        let interpreter = CompilerInterpreter::new_with_config(&[]);

        // Test that cc1 is recognized and routed to GccInterpreter, then ignored
        let cc1_execution = Execution::from_strings(
            "/usr/libexec/gcc/x86_64-linux-gnu/11/cc1",
            vec!["cc1", "-quiet", "test.c"],
            "/home/user",
            std::collections::HashMap::new(),
        );

        let result = interpreter.recognize(&cc1_execution);
        assert!(result.is_some());

        if let Some(Command::Ignored(reason)) = result {
            assert_eq!(reason, "GCC internal executable");
        } else {
            panic!("Expected ignored command for cc1, got: {:?}", result);
        }

        // Test that cc1plus is recognized and routed to GccInterpreter, then ignored
        let cc1plus_execution = Execution::from_strings(
            "/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus",
            vec!["cc1plus", "-quiet", "test.cpp"],
            "/home/user",
            std::collections::HashMap::new(),
        );

        let result = interpreter.recognize(&cc1plus_execution);
        assert!(result.is_some());

        if let Some(Command::Ignored(reason)) = result {
            assert_eq!(reason, "GCC internal executable");
        } else {
            panic!("Expected ignored command for cc1plus, got: {:?}", result);
        }

        // Test that regular gcc commands still work normally
        let gcc_execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "-O2", "main.c"],
            "/home/user",
            std::collections::HashMap::new(),
        );

        let result = interpreter.recognize(&gcc_execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(_)) = result {
            // This is expected - regular gcc commands should be processed as compiler commands
        } else {
            panic!(
                "Expected compiler command for regular gcc, got: {:?}",
                result
            );
        }
    }

    #[test]
    fn test_compiler_type_delegation_separation() {
        let interpreter = CompilerInterpreter::default();

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

    #[test]
    fn test_end_to_end_config_based_compiler_hints() {
        // Simulate a configuration where users have specified custom compiler wrappers
        // with explicit type hints that override automatic detection
        let compilers = vec![
            Compiler {
                path: PathBuf::from("custom-gcc-wrapper"),
                as_: Some(CompilerType::Gcc),
                ignore: false,
                flags: None,
            },
            Compiler {
                path: PathBuf::from("weird-clang-binary"),
                as_: Some(CompilerType::Clang),
                ignore: false,
                flags: None,
            },
            Compiler {
                path: PathBuf::from("intel-fortran-custom"),
                as_: Some(CompilerType::IntelFortran),
                ignore: false,
                flags: None,
            },
            // This one has no hint, should use auto-detection
            Compiler {
                path: PathBuf::from("gcc"),
                as_: None,
                ignore: false,
                flags: None,
            },
        ];

        // Create interpreter with configuration-based hints
        let interpreter = CompilerInterpreter::new_with_config(&compilers);

        // Test 1: Custom GCC wrapper should be recognized as GCC due to hint
        let gcc_execution = create_execution(
            "custom-gcc-wrapper",
            vec!["custom-gcc-wrapper", "-c", "-o", "output.o", "input.c"],
            "/project",
        );
        let gcc_result = interpreter.recognize(&gcc_execution);
        assert!(
            gcc_result.is_some(),
            "Custom GCC wrapper should be recognized due to configuration hint"
        );

        // Test 2: Weird Clang binary should be recognized as Clang due to hint
        let clang_execution = create_execution(
            "weird-clang-binary",
            vec!["weird-clang-binary", "-c", "-o", "output.o", "input.cpp"],
            "/project",
        );
        let clang_result = interpreter.recognize(&clang_execution);
        assert!(
            clang_result.is_some(),
            "Weird Clang binary should be recognized due to configuration hint"
        );

        // Test 3: Intel Fortran custom should be recognized due to hint
        let intel_execution = create_execution(
            "intel-fortran-custom",
            vec!["intel-fortran-custom", "-c", "-o", "output.o", "input.f90"],
            "/project",
        );
        let intel_result = interpreter.recognize(&intel_execution);
        assert!(
            intel_result.is_some(),
            "Intel Fortran custom should be recognized due to configuration hint"
        );

        // Test 4: Regular gcc should still work with auto-detection (no hint configured)
        let regular_gcc_execution = create_execution(
            "gcc",
            vec!["gcc", "-c", "-o", "output.o", "input.c"],
            "/project",
        );
        let regular_gcc_result = interpreter.recognize(&regular_gcc_execution);
        assert!(
            regular_gcc_result.is_some(),
            "Regular GCC should be auto-detected even without configuration hint"
        );

        // Test 5: Unknown compiler should not be recognized
        let unknown_execution = create_execution(
            "unknown-compiler",
            vec!["unknown-compiler", "input.file"],
            "/project",
        );
        let unknown_result = interpreter.recognize(&unknown_execution);
        assert!(
            unknown_result.is_none(),
            "Unknown compiler should not be recognized"
        );
    }

    #[test]
    fn test_recognizer_hint_priority_over_regex() {
        // Test that configuration hints take priority over regex-based detection
        let compilers = vec![
            // This would normally be detected as GCC by regex, but we force it to be Clang
            Compiler {
                path: PathBuf::from("gcc"),
                as_: Some(CompilerType::Clang),
                ignore: false,
                flags: None,
            },
        ];

        let recognizer = CompilerRecognizer::new_with_config(&compilers);

        // The hint should override regex detection
        assert_eq!(
            recognizer.recognize(Path::new("gcc")),
            Some(CompilerType::Clang),
            "Configuration hint should override regex-based detection"
        );
    }

    #[test]
    fn test_path_canonicalization_in_hints() {
        // Test that path canonicalization works correctly for hints
        // This is important for matching paths that might be specified differently
        // in config vs. what appears in execution
        let compilers = vec![Compiler {
            path: PathBuf::from("./custom-compiler"),
            as_: Some(CompilerType::Gcc),
            ignore: false,
            flags: None,
        }];

        let recognizer = CompilerRecognizer::new_with_config(&compilers);

        // Should have a hint (even though canonicalization might change the path)
        assert_eq!(
            recognizer.recognize(Path::new("./custom-compiler")),
            Some(CompilerType::Gcc)
        );
    }

    #[test]
    fn test_mixed_configuration_scenario() {
        // Test a realistic scenario with a mix of hints and auto-detection
        let compilers = vec![
            // Build system uses custom wrapper for cross-compilation
            Compiler {
                path: PathBuf::from("arm-linux-gnueabi-gcc-wrapper"),
                as_: Some(CompilerType::Gcc),
                ignore: false,
                flags: None,
            },
            // Custom Clang wrapper that doesn't follow naming conventions
            Compiler {
                path: PathBuf::from("project-clang"),
                as_: Some(CompilerType::Clang),
                ignore: false,
                flags: None,
            },
            // Standard compilers with no hints - should auto-detect
            Compiler {
                path: PathBuf::from("clang-15"),
                as_: None, // Should auto-detect as Clang
                ignore: false,
                flags: None,
            },
            Compiler {
                path: PathBuf::from("gfortran"),
                as_: None, // Should auto-detect as Fortran
                ignore: false,
                flags: None,
            },
        ];

        let interpreter = CompilerInterpreter::new_with_config(&compilers);

        // Test the cross-compilation wrapper (with hint)
        let cross_gcc = create_execution(
            "arm-linux-gnueabi-gcc-wrapper",
            vec!["arm-linux-gnueabi-gcc-wrapper", "-c", "test.c"],
            "/project",
        );
        assert!(interpreter.recognize(&cross_gcc).is_some());

        // Test the custom Clang wrapper (with hint)
        let custom_clang = create_execution(
            "project-clang",
            vec!["project-clang", "-c", "test.cpp"],
            "/project",
        );
        assert!(interpreter.recognize(&custom_clang).is_some());

        // Test standard compilers (auto-detection)
        let standard_clang =
            create_execution("clang-15", vec!["clang-15", "-c", "test.c"], "/project");
        assert!(interpreter.recognize(&standard_clang).is_some());

        let standard_fortran =
            create_execution("gfortran", vec!["gfortran", "-c", "test.f90"], "/project");
        assert!(interpreter.recognize(&standard_fortran).is_some());
    }

    #[test]
    fn test_hint_validation_warnings() {
        // Test the hint validation functionality that can be used to warn users
        // about potential misconfigurations
        let compilers = vec![
            // Correctly hinted compiler
            Compiler {
                path: PathBuf::from("gcc"),
                as_: Some(CompilerType::Gcc),
                ignore: false,
                flags: None,
            },
            // Incorrectly hinted compiler (would be detected as GCC but hinted as Clang)
            Compiler {
                path: PathBuf::from("g++"),
                as_: Some(CompilerType::Clang),
                ignore: false,
                flags: None,
            },
        ];

        let recognizer = CompilerRecognizer::new_with_config(&compilers);

        // Validation should pass for correctly hinted compiler
        assert_eq!(
            recognizer.recognize(Path::new("gcc")),
            Some(CompilerType::Gcc),
            "Validation should pass when hint matches auto-detection"
        );

        // But the hint should still take priority in actual recognition
        assert_eq!(
            recognizer.recognize(Path::new("g++")),
            Some(CompilerType::Clang),
            "Hint should take priority even when it conflicts with auto-detection"
        );
    }
}

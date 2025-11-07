// SPDX-License-Identifier: GPL-3.0-or-later

//! Clang command-line argument parser for compilation database generation.
//!
//! This module provides a specialized interpreter for parsing Clang and LLVM-based
//! compiler command lines. It builds upon GCC flag definitions and extends them with
//! Clang-specific flags, taking advantage of Clang's design goal of GCC compatibility.
//!
//! The interpreter recognizes various compiler flags and categorizes them into semantic
//! groups (source files, output files, compilation options, etc.) to generate accurate
//! compilation database entries for Clang-based toolchains.

use super::super::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use super::gcc::{parse_arguments_and_environment, GCC_FLAGS};
use crate::semantic::{
    ArgumentKind, Command, CompilerCommand, CompilerPass, Execution, Interpreter,
};

/// Clang command-line argument parser that extracts semantic information from compiler invocations.
///
/// This interpreter processes Clang and LLVM-based compiler command lines to identify:
/// - Source files being compiled
/// - Output files and directories
/// - Compiler flags that affect compilation
/// - Include directories and preprocessor definitions
/// - Clang-specific features like sanitizers, static analysis, and LLVM passes
///
/// It extends GCC flag definitions with Clang-specific flags, leveraging Clang's
/// GCC compatibility while supporting Clang's unique features and syntax variations.
pub struct ClangInterpreter {
    /// Flag analyzer that recognizes and categorizes Clang command-line flags
    /// (includes GCC-compatible flags plus Clang-specific extensions)
    matcher: FlagAnalyzer,
}

impl Default for ClangInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl ClangInterpreter {
    /// Creates a new Clang interpreter with comprehensive Clang flag definitions.
    ///
    /// The interpreter is configured with patterns to recognize both GCC-compatible flags
    /// and Clang-specific extensions including sanitizers, static analysis options,
    /// LLVM optimization passes, and Clang's unique command-line syntax variations.
    pub fn new() -> Self {
        Self {
            matcher: FlagAnalyzer::new(&CLANG_FLAGS),
        }
    }
}

impl Interpreter for ClangInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // Parse both command-line arguments and environment variables
        let annotated_args = parse_arguments_and_environment(&self.matcher, execution);

        Some(Command::Compiler(CompilerCommand::new(
            execution.working_dir.clone(),
            execution.executable.clone(),
            annotated_args,
        )))
    }
}

/// Clang flag definitions using pattern matching for argument parsing (extends GCC)
///
/// https://clang.llvm.org/docs/ClangCommandLineReference.html
static CLANG_FLAGS: std::sync::LazyLock<Vec<FlagRule>> = std::sync::LazyLock::new(|| {
    // Start with Clang-specific flags first to give them priority over GCC prefix matchers
    let mut flags = vec![
        // Clang target specification (not covered by any GCC prefix)
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--target"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-target", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-triple", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang static analysis (not covered by GCC)
        FlagRule::new(
            FlagPattern::Exactly("--analyze", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Xanalyzer", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-analyzer-config", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang code generation (not covered by -f prefix due to no dash)
        FlagRule::new(
            FlagPattern::Exactly("-emit-llvm", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang resource directory (not covered by any prefix)
        FlagRule::new(
            FlagPattern::Exactly("-resource-dir", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang compilation database (not covered by any prefix)
        FlagRule::new(
            FlagPattern::Exactly("-MJ", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        // Clang CUDA support (--cuda-* not covered by any prefix)
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--cuda-path"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--cuda-gpu-arch"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang HIP support (not covered by any prefix)
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--hip-path"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang cross-compilation toolchain support (not covered by any prefix)
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--gcc-toolchain"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--gcc-install-dir"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang plugin support (not covered by any prefix)
        FlagRule::new(
            FlagPattern::Exactly("-load", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-plugin", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-plugin-arg-", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Clang framework support (macOS/iOS) - -F not covered by GCC
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-F"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-framework", 1),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        // Clang LLVM option passing (must be before -m prefix matcher)
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-mllvm"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
    ];

    // Add GCC-compatible flags as the base (after Clang-specific ones for priority)
    flags.extend(GCC_FLAGS.iter().cloned());

    // Sort by flag length descending to ensure longer matches are tried first
    flags.sort_by(|a, b| b.pattern.flag().len().cmp(&a.pattern.flag().len()));

    flags
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::ArgumentKind;
    use std::borrow::Cow;
    use std::collections::HashMap;

    fn create_execution(executable: &str, args: Vec<&str>, working_dir: &str) -> Execution {
        Execution::from_strings(executable, args, working_dir, HashMap::new())
    }

    fn create_execution_with_env(
        executable: &str,
        args: Vec<&str>,
        working_dir: &str,
        environment: HashMap<&str, &str>,
    ) -> Execution {
        Execution::from_strings(executable, args, working_dir, environment)
    }

    /// Creates a platform-specific path string from individual path components.
    /// On Windows, paths are separated by semicolons; on Unix-like systems, by colons.
    fn create_path_string(paths: &[&str]) -> String {
        let path_bufs: Vec<std::path::PathBuf> =
            paths.iter().map(std::path::PathBuf::from).collect();
        std::env::join_paths(path_bufs)
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_simple_clang_compilation() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution("clang", vec!["clang", "-c", "-O2", "main.c"], "/project");

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            // Check compiler
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            // Check -c flag
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Check -O2 flag
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Check source file
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_specific_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang++",
            vec![
                "clang++",
                "-Weverything",
                "--target",
                "x86_64-apple-darwin",
                "-fsanitize=address",
                "-std=c++20",
                "main.cpp",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            // Check compiler
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            // Check -Weverything flag (Clang-specific)
            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(None));

            // Check --target flag (separate form)
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["--target", "x86_64-apple-darwin"]
            );

            // Check source file
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_optimization_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-O3",
                "-flto",
                "-fsave-optimization-record",
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 5);

            // All flags should be recognized as compilation flags
            for i in 1..4 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }

            // Check source file
            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_target_flag_variations() {
        let interpreter = ClangInterpreter::new();

        // Test --target form
        let execution = create_execution(
            "clang",
            vec!["clang", "--target", "arm64-apple-macos", "main.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 3);
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["--target", "arm64-apple-macos"]
            );
        }

        // Test -target form
        let execution = create_execution(
            "clang",
            vec!["clang", "-target", "arm64-apple-macos", "main.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 3);
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-target", "arm64-apple-macos"]
            );
        }
    }

    #[test]
    fn test_sanitizer_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-fsanitize=address,undefined",
                "-fsanitize-recover=unsigned-integer-overflow",
                "-fsanitize-ignorelist=mylist.txt",
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 5);

            // All sanitizer flags should be recognized
            for i in 1..4 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }

            // Check source file
            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_mllvm_flag() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-O2",
                "-mllvm",
                "-inline-threshold=100",
                "myfile.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            // Check -O2 flag
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Check -mllvm flag
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-mllvm", "-inline-threshold=100"]
            );

            // Check source file
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_mllvm_flag_equals_form() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec!["clang", "-O2", "-mllvm=-inline-threshold=100", "myfile.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            // Check -O2 flag
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Check -mllvm flag with equals form
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-mllvm=-inline-threshold=100"]
            );

            // Check source file
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_argument_parsing_with_any_executable() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "/usr/bin/clang++",
            vec!["/usr/bin/clang++", "-std=c++17", "-Wall", "test.cpp"],
            "/home/user",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Other(None));
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_comprehensive_flag_coverage() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-c",
                "-Wall",
                "-Weverything", // Clang-specific
                "-O2",
                "-g",
                "-fmodules",           // Clang-specific
                "-fcolor-diagnostics", // Clang-specific
                "-I/usr/include",
                "-D_GNU_SOURCE",
                "--target=x86_64-linux-gnu", // Clang-specific
                "-fsanitize=address",        // Enhanced in Clang
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 13);

            // All flags should be properly recognized
            for i in 1..12 {
                match cmd.arguments[i].kind() {
                    ArgumentKind::Other(Some(_)) => {} // Expected for compilation flags
                    ArgumentKind::Other(None) => {} // Expected for warning flags like -Wall, -Weverything
                    other => panic!("Unexpected argument kind at index {}: {:?}", i, other),
                }
            }

            // Check source file
            assert_eq!(cmd.arguments[12].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_cross_compilation_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "--target=aarch64-linux-gnu",
                "--gcc-toolchain=/opt/gcc-cross",
                "--gcc-install-dir=/opt/gcc",
                "-triple",
                "arm64-apple-ios",
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            // All cross-compilation flags should be recognized
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_cuda_and_openmp_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "--cuda-path=/usr/local/cuda",
                "--cuda-gpu-arch=sm_70",
                "-fcuda-rdc",
                "-fopenmp",
                "-fopenmp-targets=nvptx64",
                "main.cu",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 7);

            // All CUDA and OpenMP flags should be recognized
            for i in 1..6 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }

            // Check source file
            assert_eq!(cmd.arguments[6].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_framework_and_plugin_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-F/System/Library/Frameworks",
                "-framework",
                "Foundation",
                "-load",
                "/path/to/plugin.so",
                "-plugin",
                "my-plugin",
                "main.m",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            // Framework flag (-F/System/Library/Frameworks)
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Framework name (-framework Foundation)
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
            );

            // Plugin flags (-load and -plugin)
            for i in 3..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }

            // Source file
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_analysis_and_codegen_flags() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "--analyze",
                "-Xanalyzer",
                "-analyzer-output=text",
                "-emit-llvm",
                "-fprofile-instr-generate",
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            // Analysis flags
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Codegen flags
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Check source file
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_compilation_database_flag() {
        let interpreter = ClangInterpreter::new();

        let execution = create_execution(
            "clang",
            vec!["clang", "-MJ", "compile_commands.json", "main.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 3);

            // Check -MJ flag (Clang-specific compilation database)
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-MJ", "compile_commands.json"]
            );

            // Check source file
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_flag_inheritance() {
        // Test that Clang flags include all GCC flags plus Clang-specific extensions
        let gcc_flags = &GCC_FLAGS;
        let clang_flags = &CLANG_FLAGS;

        // Clang should have more flags than GCC (inheritance + extensions)
        assert!(
            clang_flags.len() > gcc_flags.len(),
            "Clang should have more flags than GCC, got gcc: {}, clang: {}",
            gcc_flags.len(),
            clang_flags.len()
        );

        // Create sets of flag strings for comparison
        let gcc_flag_strings: std::collections::HashSet<&str> =
            gcc_flags.iter().map(|f| f.pattern.flag()).collect();

        let clang_flag_strings: std::collections::HashSet<&str> =
            clang_flags.iter().map(|f| f.pattern.flag()).collect();

        // Verify that all GCC flags are present in Clang flags
        let missing_flags: Vec<&str> = gcc_flag_strings
            .difference(&clang_flag_strings)
            .cloned()
            .collect();

        assert!(
            missing_flags.is_empty(),
            "These GCC flags are missing from Clang: {:?}",
            missing_flags
        );

        // Find Clang-specific flags (flags in Clang but not in GCC)
        let clang_specific: Vec<&str> = clang_flag_strings
            .difference(&gcc_flag_strings)
            .cloned()
            .collect();

        // Verify some expected Clang-specific flags are present
        let expected_clang_flags = vec!["--target", "-target", "-triple", "--analyze", "-MJ"];

        for expected in expected_clang_flags {
            assert!(
                clang_specific.contains(&expected),
                "Expected Clang-specific flag '{}' not found in Clang-specific flags",
                expected
            );
        }

        println!(
            "✅ GCC flag inheritance verified: {} GCC flags + {} Clang-specific = {} total",
            gcc_flags.len(),
            clang_specific.len(),
            clang_flags.len()
        );
    }

    #[test]
    fn test_removed_flags_covered_by_prefix_matchers() {
        // Test that flags we removed from Clang extensions are still handled by GCC prefix matchers
        let interpreter = ClangInterpreter::new();

        // Test -f* flags (covered by GCC's -f prefix matcher)
        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-fsanitize=address",       // Removed from Clang extensions
                "-fmodules",                // Removed from Clang extensions
                "-fcolor-diagnostics",      // Removed from Clang extensions
                "-fprofile-instr-generate", // Removed from Clang extensions
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            // All -f* flags should still be recognized via GCC's prefix matcher
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling)),
                    "Flag at index {} should be recognized by -f prefix matcher",
                    i
                );
            }
        } else {
            panic!("Expected compiler command");
        }

        // Test -W* flags (covered by GCC's -W prefix matcher)
        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-Weverything",    // Removed from Clang extensions
                "-Wno-everything", // Removed from Clang extensions
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            // All -W* flags should still be recognized via GCC's prefix matcher
            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(None));
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Other(None));
        } else {
            panic!("Expected compiler command");
        }

        // Test -m* flags (covered by GCC's -m prefix matcher)
        let execution = create_execution(
            "clang",
            vec![
                "clang",
                "-mllvm", // Removed from Clang extensions
                "-enable-vectorizer",
                "main.c",
            ],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = interpreter.recognize(&execution) {
            // -mllvm should still be recognized via GCC's -m prefix matcher
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
        } else {
            panic!("Expected compiler command");
        }

        println!("✅ Verified removed flags are still handled by GCC prefix matchers");
    }

    #[test]
    fn test_environment_variables_cpath() {
        let interpreter = ClangInterpreter::new();
        let cpath = create_path_string(&["/usr/include", "/opt/include"]);
        let mut env = HashMap::new();
        env.insert("CPATH", cpath.as_str());

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have original 4 args + 2 include directories (each as single arg containing -I and path)
            assert_eq!(cmd.arguments.len(), 6);

            // Check that environment includes are added
            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            assert!(args_as_strings.contains(&"-I".to_string()));
            assert!(args_as_strings.contains(&"/usr/include".to_string()));
            assert!(args_as_strings.contains(&"/opt/include".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_environment_variables_c_include_path() {
        let interpreter = ClangInterpreter::new();
        let mut env = HashMap::new();
        env.insert("C_INCLUDE_PATH", "/usr/local/include");

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have original 4 args + 1 include directory (as single arg containing -I and path)
            assert_eq!(cmd.arguments.len(), 5);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            assert!(args_as_strings.contains(&"-I".to_string()));
            assert!(args_as_strings.contains(&"/usr/local/include".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_environment_variables_cplus_include_path() {
        let interpreter = ClangInterpreter::new();
        let mut env = HashMap::new();
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");

        let execution = create_execution_with_env(
            "clang++",
            vec!["clang++", "-c", "main.cpp", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have original 4 args + 1 include directory (as single arg containing -I and path)
            assert_eq!(cmd.arguments.len(), 5);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            assert!(args_as_strings.contains(&"-I".to_string()));
            assert!(args_as_strings.contains(&"/usr/include/c++/11".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_environment_variables_multiple() {
        let interpreter = ClangInterpreter::new();
        let cpath = create_path_string(&["/usr/include", "/opt/include"]);
        let mut env = HashMap::new();
        env.insert("CPATH", cpath.as_str());
        env.insert("C_INCLUDE_PATH", "/usr/local/include");
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have original 4 args + 4 include directories (each as single arg containing -I and path)
            assert_eq!(cmd.arguments.len(), 8);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            // Check that all environment includes are added
            assert!(args_as_strings.contains(&"/usr/include".to_string()));
            assert!(args_as_strings.contains(&"/opt/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/local/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/include/c++/11".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_environment_variables_empty_paths() {
        let interpreter = ClangInterpreter::new();
        let c_include_path = create_path_string(&["", "", "", ""]);
        let mut env = HashMap::new();
        env.insert("CPATH", "");
        env.insert("C_INCLUDE_PATH", c_include_path.as_str()); // Empty paths should be filtered out

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have only the original 4 args, no additional includes
            assert_eq!(cmd.arguments.len(), 4);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_environment_variables_objc_include_path() {
        let interpreter = ClangInterpreter::new();
        let objc_include_path =
            create_path_string(&["/System/Library/Frameworks", "/usr/local/objc"]);
        let mut env = HashMap::new();
        env.insert("OBJC_INCLUDE_PATH", objc_include_path.as_str());

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.m", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have original 4 args + 2 system include directories (each as single arg containing -isystem and path)
            assert_eq!(cmd.arguments.len(), 6);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            assert!(args_as_strings.contains(&"-isystem".to_string()));
            assert!(args_as_strings.contains(&"/System/Library/Frameworks".to_string()));
            assert!(args_as_strings.contains(&"/usr/local/objc".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_environment_variables_all_types() {
        let interpreter = ClangInterpreter::new();
        let mut env = HashMap::new();
        env.insert("CPATH", "/usr/include");
        env.insert("C_INCLUDE_PATH", "/usr/local/include");
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");
        env.insert("OBJC_INCLUDE_PATH", "/System/Library/Frameworks");

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should have original 4 args + 4 include directories (each as single arg)
            assert_eq!(cmd.arguments.len(), 8);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            // Check that all environment includes are added with correct flags
            assert!(args_as_strings.contains(&"/usr/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/local/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/include/c++/11".to_string()));
            assert!(args_as_strings.contains(&"/System/Library/Frameworks".to_string()));
            assert!(args_as_strings.contains(&"-I".to_string()));
            assert!(args_as_strings.contains(&"-isystem".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }
}

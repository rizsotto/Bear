// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler interpreters for recognizing and parsing compiler invocations.
//!
//! This module provides interpreters for various compiler toolchains including
//! GCC, Clang, CUDA, and Fortran compilers, as well as support for compiler
//! wrappers like ccache, distcc, and sccache.

pub mod compiler_recognition;
mod flag_based;
pub mod identity;
mod print;
mod probe;
mod response_file;
mod wrapper;

pub use print::print_compilers;

use super::super::{Interpreter, RecognizeResult};
use super::combinators::OutputLogger;
use compiler_recognition::{CompilerHints, CompilerRecognizer};
use identity::CompilerType;
use intercept::Execution;
use std::collections::HashMap;

/// Main compiler interpreter that delegates to specific compiler implementations.
///
/// `recognize` runs the recognizer to classify the executable, transparently
/// peels off any wrapper layer (ccache/distcc/sccache) via [`wrapper::unwrap`],
/// then dispatches to the per-type flag interpreter.
pub struct CompilerInterpreter {
    recognizer: CompilerRecognizer,
    interpreters: HashMap<CompilerType, Box<dyn Interpreter>>,
    inline_response_files: bool,
}

impl CompilerInterpreter {
    /// Builds a fully configured compiler interpreter from the compiler hints
    /// and the two argument-formatting switches: `from_response_files` inlines
    /// `@file` references, `from_environment` folds compiler environment
    /// variables into the arguments. Registers every supported compiler type
    /// exactly once.
    pub fn new_with_format(hints: CompilerHints, from_response_files: bool, from_environment: bool) -> Self {
        let mut result = Self {
            recognizer: CompilerRecognizer::new_with_hints(hints),
            interpreters: HashMap::new(),
            inline_response_files: from_response_files,
        };
        result.register_all(from_environment);
        result
    }

    /// Convenience constructor with the default argument handling (response-file
    /// inlining off, environment-flag folding on). Used by [`Default`] and tests.
    pub fn new_with_hints(hints: CompilerHints) -> Self {
        Self::new_with_format(hints, false, true)
    }

    /// Registers every supported compiler family from the generated
    /// [`flag_based::FAMILIES`] registry, threading `from_environment` into
    /// each per-family interpreter. Adding a family is a YAML edit; nothing
    /// here changes.
    fn register_all(&mut self, from_environment: bool) {
        for def in flag_based::FAMILIES {
            self.register(CompilerType::compiler(def.id), flag_based::interpreter(def, from_environment));
        }
    }

    /// Registers an interpreter for a specific compiler type, wrapping it
    /// with [`OutputLogger`] so its recognition result is traced.
    fn register(&mut self, compiler_type: CompilerType, interpreter: impl Interpreter + 'static) {
        let logged_interpreter = OutputLogger::new(interpreter, compiler_type.to_string());
        self.interpreters.insert(compiler_type, Box::new(logged_interpreter));
    }
}

impl Default for CompilerInterpreter {
    fn default() -> Self {
        Self::new_with_hints(CompilerHints::new())
    }
}

impl Interpreter for CompilerInterpreter {
    fn recognize(&self, execution: Execution) -> RecognizeResult {
        // Classify the executable, peeling off a wrapper layer in place if
        // needed. ccache/distcc/sccache aren't real compilers; they exist
        // to carry one in argv. wrapper::unwrap returns both the unwrapped
        // execution and the inner compiler's type so we don't have to
        // re-run the recognizer after unwrapping.
        let (execution, compiler_type) = match self.recognizer.recognize(&execution.executable) {
            Some(CompilerType::Wrapper) => match wrapper::unwrap(execution, &self.recognizer) {
                Ok(unwrapped) => unwrapped,
                Err(execution) => return RecognizeResult::NotRecognized(execution),
            },
            Some(ty) => (execution, ty),
            None => return RecognizeResult::NotRecognized(execution),
        };

        // Inline @file response files before flag classification, so their
        // tokens are classified, link-stripped, and split per source like any
        // other argument. Tokenization follows the family just identified.
        let execution = if self.inline_response_files {
            response_file::expand(execution, response_file::syntax_for(compiler_type))
        } else {
            execution
        };

        match self.interpreters.get(&compiler_type) {
            Some(interpreter) => interpreter.recognize(execution),
            None => RecognizeResult::NotRecognized(execution),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{ArgumentKind, CompilerPass, PassEffect};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use ArgumentKind::*;

    /// Short alias for `Other(PassEffect::StopsAt(pass))`.
    fn stops_at(pass: CompilerPass) -> ArgumentKind {
        Other(PassEffect::StopsAt(pass))
    }
    /// Short alias for `Other(PassEffect::Configures(pass))`.
    fn configures(pass: CompilerPass) -> ArgumentKind {
        Other(PassEffect::Configures(pass))
    }
    /// Short alias for `Other(PassEffect::None)`.
    fn none() -> ArgumentKind {
        Other(PassEffect::None)
    }
    /// Short alias for `Other(PassEffect::DriverOption)`.
    fn driver() -> ArgumentKind {
        Other(PassEffect::DriverOption)
    }
    /// Short alias for `Other(PassEffect::InfoAndExit)`.
    fn info() -> ArgumentKind {
        Other(PassEffect::InfoAndExit)
    }

    fn assert_command(result: RecognizeResult, expected: Vec<(ArgumentKind, Vec<&str>)>) {
        let RecognizeResult::Recognized(cmd) = result else {
            panic!("Expected Recognized, got {:?}", result);
        };
        let actual: Vec<(ArgumentKind, Vec<String>)> =
            cmd.arguments.iter().map(|a| (a.kind(), a.as_arguments(&|p| Cow::Borrowed(p)))).collect();
        let expected: Vec<(ArgumentKind, Vec<String>)> =
            expected.into_iter().map(|(k, args)| (k, args.into_iter().map(String::from).collect())).collect();
        assert_eq!(actual, expected);
    }

    fn assert_ignored(result: RecognizeResult, expected_reason: &str) {
        let RecognizeResult::Ignored(reason) = result else {
            panic!("Expected Ignored, got {:?}", result);
        };
        assert_eq!(reason, expected_reason);
    }

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

    fn create_path_string(paths: &[&str]) -> String {
        let path_bufs: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
        std::env::join_paths(path_bufs).unwrap().to_string_lossy().to_string()
    }

    fn noop(path: &Path) -> Cow<'_, Path> {
        Cow::from(path)
    }

    mod structural {
        use super::*;

        #[test]
        fn gcc_recognition_and_delegation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("/usr/bin/gcc", vec!["/usr/bin/gcc", "-c", "test.c"], "/tmp");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)), "GCC command should be recognized");
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.executable, PathBuf::from("/usr/bin/gcc"));
                assert_eq!(cmd.working_dir, PathBuf::from("/tmp"));
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn clang_recognition_and_delegation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("clang", vec!["clang", "-c", "main.c", "-o", "main.o"], "/tmp");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)), "Clang command should be recognized");
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.executable, PathBuf::from("clang"));
                assert_eq!(cmd.working_dir, PathBuf::from("/tmp"));
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn unrecognized_compiler() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("unknown_compiler", vec!["unknown_compiler", "-c", "test.c"], "/tmp");
            assert!(
                matches!(sut.recognize(execution), RecognizeResult::NotRecognized(_)),
                "Unknown compiler should not be recognized"
            );
        }

        #[test]
        fn delegation_preserves_execution_details() {
            let sut = CompilerInterpreter::default();
            let working_dir = PathBuf::from("/custom/working/dir");
            let mut environment = std::collections::HashMap::new();
            environment.insert("CC".to_string(), "gcc".to_string());
            let execution = Execution {
                executable: PathBuf::from("gcc"),
                arguments: vec!["gcc".to_string(), "-c".to_string(), "file.c".to_string()],
                working_dir: working_dir.clone(),
                environment,
            };
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)), "Command should be recognized");
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.working_dir, working_dir);
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn end_to_end_config_based_compiler_hints() {
            let mut hints = CompilerHints::new();
            hints.add(Path::new("/custom/path/my-gcc"), Some("gcc")).unwrap();
            hints.add(Path::new("/opt/clang/bin/clang++"), Some("clang")).unwrap();

            let sut = CompilerInterpreter::new_with_hints(hints);
            let custom_gcc =
                create_execution("/custom/path/my-gcc", vec!["/custom/path/my-gcc", "-c", "test.c"], "/tmp");
            assert!(
                matches!(sut.recognize(custom_gcc), RecognizeResult::Recognized(_)),
                "Custom GCC path should be recognized via config hint"
            );
            let custom_clang = create_execution(
                "/opt/clang/bin/clang++",
                vec!["/opt/clang/bin/clang++", "-c", "main.cpp"],
                "/tmp",
            );
            assert!(
                matches!(sut.recognize(custom_clang), RecognizeResult::Recognized(_)),
                "Custom Clang path should be recognized via config hint"
            );
            let normal_gcc = create_execution("gcc", vec!["gcc", "-c", "normal.c"], "/tmp");
            assert!(
                matches!(sut.recognize(normal_gcc), RecognizeResult::Recognized(_)),
                "Standard GCC should still be recognized"
            );
        }

        // Requirements: recognition-compiler-launchers
        #[test]
        fn wrapper_recognition_and_delegation() {
            let sut = CompilerInterpreter::default();
            let ccache_execution = create_execution("ccache", vec!["ccache", "gcc", "-c", "test.c"], "/tmp");
            let result = sut.recognize(ccache_execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(*cmd.executable, *"gcc");
                let arguments: Vec<String> =
                    cmd.arguments.into_iter().flat_map(|arg| arg.as_arguments(&noop)).collect();
                assert_eq!(vec!["gcc".to_string(), "-c".to_string(), "test.c".to_string()], arguments);
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn uniform_delegation() {
            let sut = CompilerInterpreter::default();
            let executables = vec!["gcc", "clang", "nvcc", "gfortran", "ifort"];
            for executable in executables {
                let execution = create_execution(executable, vec![executable, "-c", "test.c"], "/tmp");
                let recognized_type = sut.recognizer.recognize(&execution.executable);
                if let Some(compiler_type) = recognized_type {
                    assert!(
                        sut.interpreters.contains_key(&compiler_type),
                        "Interpreter should be registered for {}",
                        executable
                    );
                }
            }
        }

        /// Data-driven completeness check over every row of the codegen-generated
        /// `RECOGNITION_PATTERNS` table (see `compiler_recognition.rs`): for the
        /// first executable name in each row, dispatch through the full
        /// `CompilerInterpreter` (recognizer + wrapper-unwrap + registered
        /// interpreter), not just the recognizer.
        ///
        /// Why this matters: recognition and registration are generated from
        /// the same compiler-definition data, but through separate paths (the
        /// `RECOGNITION_PATTERNS` table and the `FAMILIES` registry). If those
        /// two ever disagree -- a recognized id with no registry row -- every
        /// recognition unit test still passes (the regex matches and
        /// `recognize()` returns `Some(type)`), but `CompilerInterpreter` has no
        /// interpreter for that type, so every real execution of the family
        /// silently falls through to `NotRecognized` -- the same failure mode
        /// as not recognizing the compiler at all, just discovered later and
        /// more confusingly. This test catches that gap for every family
        /// automatically, current and future, without needing a new test per
        /// family.
        ///
        /// `NotRecognized` is the only failure signal: `Ignored` also proves
        /// registration ran (the internal-executable rows -- cc1, c1, etc. --
        /// legitimately resolve there via `ignore_when`), so the assertion is
        /// "not NotRecognized", not "Recognized".
        #[test]
        fn every_recognition_pattern_row_is_dispatched_by_a_registered_interpreter() {
            let sut = CompilerInterpreter::default();

            for &(type_str, executables, _cross_compilation, _versioned, _description) in
                compiler_recognition::RECOGNITION_PATTERNS
            {
                let name = executables[0];
                // Every family dispatches on a plain "-c hello.c" invocation
                // regardless of the source extension: source-vs-flag
                // classification in FlagBasedInterpreter never inspects the
                // file extension (verified empirically against vala, cuda,
                // and the fortran families -- all still recognize a `.c`
                // source; extension only affects the binary-vs-source flag on
                // an already-classified Source argument, not the RecognizeResult
                // variant). Wrapper rows are the one exception: `-c` is not a
                // real compiler name, so a wrapper's own unwrap logic would
                // resolve NotRecognized on that invocation; dispatch a full
                // valid wrapper invocation instead (`<name> gcc -c hello.c`),
                // which also proves every wrapper basename unwraps to a
                // registered inner compiler.
                let args = if type_str == "wrapper" {
                    vec![name, "gcc", "-c", "hello.c"]
                } else {
                    vec![name, "-c", "hello.c"]
                };
                let execution = create_execution(name, args, "/project");
                let result = sut.recognize(execution);
                assert!(
                    !matches!(result, RecognizeResult::NotRecognized(_)),
                    "family '{}' (executable '{}') matched the recognizer but was not dispatched \
                     by any registered interpreter -- its id is in RECOGNITION_PATTERNS but not in \
                     the generated FAMILIES registry",
                    type_str,
                    name
                );
            }
        }
    }

    mod gcc {
        use super::*;

        #[test]
        fn simple_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project");
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (Source { binary: false }, vec!["main.c"]),
                    (Output, vec!["-o", "main.o"]),
                ],
            );
        }

        #[test]
        fn combined_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec!["gcc", "-I/usr/include", "-DDEBUG=1", "-o", "main", "main.c"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (configures(CompilerPass::Preprocessing), vec!["-I/usr/include"]),
                    (configures(CompilerPass::Preprocessing), vec!["-DDEBUG=1"]),
                    (Output, vec!["-o", "main"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn separate_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec!["gcc", "-I", "/usr/include", "-D", "DEBUG=1", "main.c"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (configures(CompilerPass::Preprocessing), vec!["-I", "/usr/include"]),
                    (configures(CompilerPass::Preprocessing), vec!["-D", "DEBUG=1"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn response_file() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("gcc", vec!["gcc", "@response.txt", "main.c"], "/project");
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (configures(CompilerPass::Compiling), vec!["@response.txt"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn warning_flags() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("gcc", vec!["gcc", "-Wall", "-Wextra", "-Wno-unused", "main.c"], "/project");
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (none(), vec!["-Wall"]),
                    (none(), vec!["-Wextra"]),
                    (none(), vec!["-Wno-unused"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn std_flag_variations() {
            let sut = CompilerInterpreter::default();

            for (args, expected_flag_args) in [
                (vec!["gcc", "-std", "c99", "main.c"], vec!["-std", "c99"]),
                (vec!["gcc", "-std=c99", "main.c"], vec!["-std=c99"]),
            ] {
                let execution = create_execution("gcc", args, "/project");
                let result = sut.recognize(execution);
                if let RecognizeResult::Recognized(cmd) = result {
                    assert_eq!(cmd.arguments[1].kind(), configures(CompilerPass::Compiling));
                    assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), expected_flag_args);
                }
            }
        }

        #[test]
        fn complex_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "-Wall",
                    "-Werror",
                    "-O2",
                    "-g",
                    "-I/usr/local/include",
                    "-I",
                    "/opt/include",
                    "-DVERSION=1.0",
                    "-D",
                    "DEBUG",
                    "-fPIC",
                    "-m64",
                    "-c",
                    "main.c",
                    "utils.c",
                    "-o",
                    "program",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert!(cmd.arguments.len() >= 10);
                let source_count = cmd.arguments.iter().filter(|a| matches!(a.kind(), Source { .. })).count();
                let output_count = cmd.arguments.iter().filter(|a| a.kind() == Output).count();
                assert_eq!(source_count, 2);
                assert_eq!(output_count, 1);
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn comprehensive_flag_coverage() {
            let sut = CompilerInterpreter::default();

            // (flags, expected_kind) for prefix-matching flag groups
            let cases: Vec<(Vec<&str>, ArgumentKind)> = vec![
                (vec!["-O2", "-Os", "-Ofast", "-Og"], configures(CompilerPass::Compiling)),
                (vec!["-g", "-g3", "-gdwarf-4", "-ggdb"], configures(CompilerPass::Compiling)),
                (vec!["-Wall", "-Wextra", "-Wno-unused", "-Werror=format"], none()),
                (
                    vec!["-fPIC", "-fstack-protector", "-fno-omit-frame-pointer", "-flto"],
                    configures(CompilerPass::Compiling),
                ),
                (
                    vec!["-m64", "-march=native", "-mtune=generic", "-msse4.2"],
                    configures(CompilerPass::Compiling),
                ),
            ];

            for (flags, expected_kind) in cases {
                let mut args = vec!["gcc"];
                args.extend(&flags);
                args.push("main.c");
                let execution = create_execution("gcc", args, "/project");
                let result = sut.recognize(execution);
                if let RecognizeResult::Recognized(cmd) = result {
                    assert!(cmd.arguments.len() > flags.len());
                    for i in 1..=flags.len() {
                        assert_eq!(cmd.arguments[i].kind(), expected_kind, "flag: {}", flags[i - 1]);
                    }
                }
            }
        }

        #[test]
        fn linker_and_system_flags() {
            let sut = CompilerInterpreter::default();

            // Test linker flags
            let execution = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "-Wl,--gc-sections",
                    "-Wl,-rpath,/usr/local/lib",
                    "-static",
                    "-shared",
                    "-pie",
                    "-pthread",
                    "main.c",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                for i in 1..=3 {
                    assert_eq!(cmd.arguments[i].kind(), configures(CompilerPass::Linking));
                }
            }

            // Test system include and library paths
            let execution = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "-isystem",
                    "/usr/local/include",
                    "-L/usr/local/lib",
                    "-lmath",
                    "--sysroot=/opt/sysroot",
                    "main.c",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments[1].kind(), configures(CompilerPass::Preprocessing));
                assert_eq!(cmd.arguments[2].kind(), configures(CompilerPass::Linking));
                assert_eq!(cmd.arguments[3].kind(), configures(CompilerPass::Linking));
            }
        }

        #[test]
        fn response_files_and_plugins() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "@compile_flags.txt",
                    "-fplugin=myplugin.so",
                    "-fplugin-arg-myplugin-option=value",
                    "-save-temps=obj",
                    "main.c",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments[1].kind(), configures(CompilerPass::Compiling));
                assert_eq!(cmd.arguments[2].kind(), configures(CompilerPass::Compiling));
                assert_eq!(cmd.arguments[3].kind(), configures(CompilerPass::Compiling));
                assert_eq!(cmd.arguments[4].kind(), driver());
            }
        }

        /// Requirements: output-env-derived-flags
        #[test]
        fn environment_disabled_suppresses_injected_flags() {
            let sut = CompilerInterpreter::new_with_format(CompilerHints::new(), false, false);
            let cpath = create_path_string(&["/usr/include", "/opt/include"]);
            let mut env = HashMap::new();
            env.insert("CPATH", cpath.as_str());
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );

            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (Source { binary: false }, vec!["main.c"]),
                    (Output, vec!["-o", "main.o"]),
                ],
            );
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_cpath() {
            let sut = CompilerInterpreter::default();
            let cpath = create_path_string(&["/usr/include", "/opt/include"]);
            let mut env = HashMap::new();
            env.insert("CPATH", cpath.as_str());
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 6);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-I".to_string()));
                assert!(args.contains(&"/usr/include".to_string()));
                assert!(args.contains(&"/opt/include".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_c_include_path() {
            let sut = CompilerInterpreter::default();
            let mut env = HashMap::new();
            env.insert("C_INCLUDE_PATH", "/usr/local/include");
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 5);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-isystem".to_string()));
                assert!(args.contains(&"/usr/local/include".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_cplus_include_path() {
            let sut = CompilerInterpreter::default();
            let mut env = HashMap::new();
            env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");
            let execution = create_execution_with_env(
                "g++",
                vec!["g++", "-c", "main.cpp", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 5);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-isystem".to_string()));
                assert!(args.contains(&"/usr/include/c++/11".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_multiple() {
            let sut = CompilerInterpreter::default();
            let cpath = create_path_string(&["/usr/include", "/opt/include"]);
            let mut env = HashMap::new();
            env.insert("CPATH", cpath.as_str());
            env.insert("C_INCLUDE_PATH", "/usr/local/include");
            env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 8);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"/usr/include".to_string()));
                assert!(args.contains(&"/opt/include".to_string()));
                assert!(args.contains(&"/usr/local/include".to_string()));
                assert!(args.contains(&"/usr/include/c++/11".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_objc_include_path() {
            let sut = CompilerInterpreter::default();
            let objc_include_path = create_path_string(&["/System/Library/Frameworks", "/usr/local/objc"]);
            let mut env = HashMap::new();
            env.insert("OBJC_INCLUDE_PATH", objc_include_path.as_str());
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.m", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 6);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-isystem".to_string()));
                assert!(args.contains(&"/System/Library/Frameworks".to_string()));
                assert!(args.contains(&"/usr/local/objc".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_all_types() {
            let sut = CompilerInterpreter::default();
            let mut env = HashMap::new();
            env.insert("CPATH", "/usr/include");
            env.insert("C_INCLUDE_PATH", "/usr/local/include");
            env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");
            env.insert("OBJC_INCLUDE_PATH", "/System/Library/Frameworks");
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 8);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"/usr/include".to_string()));
                assert!(args.contains(&"/usr/local/include".to_string()));
                assert!(args.contains(&"/usr/include/c++/11".to_string()));
                assert!(args.contains(&"/System/Library/Frameworks".to_string()));
                assert!(args.contains(&"-I".to_string()));
                assert!(args.contains(&"-isystem".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn environment_variables_empty_paths() {
            let sut = CompilerInterpreter::default();
            let c_include_path = create_path_string(&["", "", "", ""]);
            let mut env = HashMap::new();
            env.insert("CPATH", "");
            env.insert("C_INCLUDE_PATH", c_include_path.as_str());
            let execution = create_execution_with_env(
                "gcc",
                vec!["gcc", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 4);
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn preprocessor_comprehensive_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "-E",
                    "-C",
                    "-CC",
                    "-P",
                    "-traditional",
                    "-trigraphs",
                    "-undef",
                    "-Wp,-MD,deps.d",
                    "-M",
                    "-MM",
                    "-MG",
                    "-MP",
                    "main.c",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                // -E, -M and -MM stop the compiler after preprocessing (the
                // latter two imply -E); the rest only configure it.
                for i in [1, 9, 10] {
                    assert_eq!(cmd.arguments[i].kind(), stops_at(CompilerPass::Preprocessing), "index {i}");
                }
                for i in (2..9).chain(11..13) {
                    assert_eq!(cmd.arguments[i].kind(), configures(CompilerPass::Preprocessing), "index {i}");
                }
            }
        }

        #[test]
        fn internal_executables_are_ignored() {
            let sut = CompilerInterpreter::default();

            let internal_cases = [
                ("/usr/libexec/gcc/x86_64-linux-gnu/11/cc1", vec!["cc1", "-quiet", "test.c"]),
                ("/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus", vec!["cc1plus", "-quiet", "test.cpp"]),
                (
                    "/usr/libexec/gcc/x86_64-linux-gnu/11/collect2",
                    vec!["collect2", "-o", "program", "main.o"],
                ),
                (
                    "/usr/libexec/gcc/x86_64-redhat-linux/15/f951",
                    vec!["f951", "fortran.f90", "-mtune=generic", "-march=x86-64", "-o", "/tmp/cc6kwJ3Y.s"],
                ),
            ];
            for (exe, args) in &internal_cases {
                let execution = create_execution(exe, args.clone(), "/home/user");
                assert_ignored(sut.recognize(execution), "internal executable");
            }

            // Regular gcc should still be recognized as a compiler
            let gcc_execution =
                create_execution("/usr/bin/gcc", vec!["gcc", "-c", "-O2", "main.c"], "/home/user");
            assert!(matches!(sut.recognize(gcc_execution), RecognizeResult::Recognized(_)));
        }

        #[test]
        fn linker_command_with_object_files() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec!["gcc", "-o", "a.out", "source1.o", "source2.o", "-lx", "-L/usr/local/lib"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["gcc"]),
                    (Output, vec!["-o", "a.out"]),
                    (Source { binary: true }, vec!["source1.o"]),
                    (Source { binary: true }, vec!["source2.o"]),
                    (configures(CompilerPass::Linking), vec!["-lx"]),
                    (configures(CompilerPass::Linking), vec!["-L/usr/local/lib"]),
                ],
            );
        }

        #[test]
        fn comprehensive_linker_scenarios() {
            let sut = CompilerInterpreter::default();

            // Mixed compilation and linking
            let execution = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "-o",
                    "myprogram",
                    "main.o",
                    "utils.o",
                    "lib.a",
                    "-L/usr/lib",
                    "-L",
                    "/opt/lib",
                    "-lmath",
                    "-l",
                    "pthread",
                    "-Wl,--as-needed",
                    "-static",
                    "-pie",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert!(cmd.arguments.len() >= 10);
                let linking_count = cmd
                    .arguments
                    .iter()
                    .filter(|a| matches!(a.kind(), Other(PassEffect::Configures(CompilerPass::Linking))))
                    .count();
                assert_eq!(linking_count, 7);
            }

            // Pure linking command
            let pure_linking = create_execution(
                "gcc",
                vec![
                    "gcc",
                    "-o",
                    "final_program",
                    "obj1.o",
                    "obj2.o",
                    "obj3.o",
                    "libstatic.a",
                    "-lssl",
                    "-lcrypto",
                    "-L/usr/local/ssl/lib",
                    "-Wl,-rpath,/usr/local/ssl/lib",
                ],
                "/build",
            );
            let result = sut.recognize(pure_linking);
            if let RecognizeResult::Recognized(cmd) = result {
                let object_files: Vec<_> = cmd
                    .arguments
                    .iter()
                    .filter(|a| {
                        let args = a.as_arguments(&|p| Cow::Borrowed(p));
                        args.len() == 1 && (args[0].ends_with(".o") || args[0].ends_with(".a"))
                    })
                    .collect();
                assert_eq!(object_files.len(), 4);
                for obj_file in object_files {
                    assert_eq!(obj_file.kind(), Source { binary: true });
                }
            }
        }

        #[test]
        fn arch_flag_preserves_argument() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "gcc",
                vec!["gcc", "-arch", "arm64", "-Wall", "-O2", "-c", "hello.c"],
                "/project",
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                let arch_arg = cmd.arguments.iter().find(|a| {
                    let tokens = a.as_arguments(&|p| Cow::Borrowed(p));
                    tokens.len() == 2 && tokens[0] == "-arch" && tokens[1] == "arm64"
                });
                assert!(arch_arg.is_some(), "-arch arm64 should be captured as a single argument pair");
                assert_eq!(arch_arg.unwrap().kind(), configures(CompilerPass::Compiling));
                let bad_source = cmd.arguments.iter().any(|a| {
                    let tokens = a.as_arguments(&|p| Cow::Borrowed(p));
                    tokens.len() == 1 && tokens[0] == "arm64"
                });
                assert!(!bad_source, "arm64 must not be misclassified as a source file");
            } else {
                panic!("Expected compiler command");
            }
        }

        // Requirements: recognition-cpp20-modules
        #[test]
        fn modules_ts_flag_recognizes_module_interface_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "g++",
                vec!["g++", "-std=c++20", "-fmodules-ts", "-c", "mod.cppm"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["g++"]),
                    (configures(CompilerPass::Compiling), vec!["-std=c++20"]),
                    (configures(CompilerPass::Compiling), vec!["-fmodules-ts"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (Source { binary: false }, vec!["mod.cppm"]),
                ],
            );
        }
    }

    mod clang {
        use super::*;

        #[test]
        fn simple_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("clang", vec!["clang", "-c", "-O2", "main.c"], "/project");
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (configures(CompilerPass::Compiling), vec!["-O2"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn specific_flags() {
            let sut = CompilerInterpreter::default();
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
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang++"]),
                    (none(), vec!["-Weverything"]),
                    (configures(CompilerPass::Compiling), vec!["--target", "x86_64-apple-darwin"]),
                    (configures(CompilerPass::Compiling), vec!["-fsanitize=address"]),
                    (configures(CompilerPass::Compiling), vec!["-std=c++20"]),
                    (Source { binary: false }, vec!["main.cpp"]),
                ],
            );
        }

        #[test]
        fn optimization_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang",
                vec!["clang", "-O3", "-flto", "-fsave-optimization-record", "main.c"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Compiling), vec!["-O3"]),
                    (configures(CompilerPass::Compiling), vec!["-flto"]),
                    (configures(CompilerPass::Compiling), vec!["-fsave-optimization-record"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn target_flag_variations() {
            let sut = CompilerInterpreter::default();

            for (args, expected_flag_args) in [
                (
                    vec!["clang", "--target", "arm64-apple-macos", "main.c"],
                    vec!["--target", "arm64-apple-macos"],
                ),
                (
                    vec!["clang", "-target", "arm64-apple-macos", "main.c"],
                    vec!["-target", "arm64-apple-macos"],
                ),
            ] {
                let execution = create_execution("clang", args, "/project");
                let result = sut.recognize(execution);
                if let RecognizeResult::Recognized(cmd) = result {
                    assert_eq!(cmd.arguments.len(), 3);
                    assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), expected_flag_args);
                }
            }
        }

        #[test]
        fn sanitizer_flags() {
            let sut = CompilerInterpreter::default();
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
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Compiling), vec!["-fsanitize=address,undefined"]),
                    (
                        configures(CompilerPass::Compiling),
                        vec!["-fsanitize-recover=unsigned-integer-overflow"],
                    ),
                    (configures(CompilerPass::Compiling), vec!["-fsanitize-ignorelist=mylist.txt"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn mllvm_flag() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang",
                vec!["clang", "-O2", "-mllvm", "-inline-threshold=100", "myfile.c"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Compiling), vec!["-O2"]),
                    (configures(CompilerPass::Compiling), vec!["-mllvm", "-inline-threshold=100"]),
                    (Source { binary: false }, vec!["myfile.c"]),
                ],
            );
        }

        #[test]
        fn mllvm_flag_equals_form() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang",
                vec!["clang", "-O2", "-mllvm=-inline-threshold=100", "myfile.c"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Compiling), vec!["-O2"]),
                    (configures(CompilerPass::Compiling), vec!["-mllvm=-inline-threshold=100"]),
                    (Source { binary: false }, vec!["myfile.c"]),
                ],
            );
        }

        #[test]
        fn comprehensive_flag_coverage() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang",
                vec![
                    "clang",
                    "-c",
                    "-Wall",
                    "-Weverything",
                    "-O2",
                    "-g",
                    "-fmodules",
                    "-fcolor-diagnostics",
                    "-I/usr/include",
                    "-D_GNU_SOURCE",
                    "--target=x86_64-linux-gnu",
                    "-fsanitize=address",
                    "main.c",
                ],
                "/project",
            );
            if let RecognizeResult::Recognized(cmd) = sut.recognize(execution) {
                assert_eq!(cmd.arguments.len(), 13);
                for i in 1..12 {
                    match cmd.arguments[i].kind() {
                        Other(PassEffect::Configures(_))
                        | Other(PassEffect::StopsAt(_))
                        | Other(PassEffect::None) => {}
                        other => panic!("Unexpected argument kind at index {}: {:?}", i, other),
                    }
                }
                assert_eq!(cmd.arguments[12].kind(), Source { binary: false });
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn cross_compilation_flags() {
            let sut = CompilerInterpreter::default();
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
            if let RecognizeResult::Recognized(cmd) = sut.recognize(execution) {
                assert_eq!(cmd.arguments.len(), 6);
                for i in 1..5 {
                    assert_eq!(cmd.arguments[i].kind(), configures(CompilerPass::Compiling));
                }
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn cuda_and_openmp_flags() {
            let sut = CompilerInterpreter::default();
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
            if let RecognizeResult::Recognized(cmd) = sut.recognize(execution) {
                assert_eq!(cmd.arguments.len(), 7);
                for i in 1..6 {
                    assert_eq!(cmd.arguments[i].kind(), configures(CompilerPass::Compiling));
                }
                assert_eq!(cmd.arguments[6].kind(), Source { binary: false });
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn framework_and_plugin_flags() {
            let sut = CompilerInterpreter::default();
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
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Compiling), vec!["-F/System/Library/Frameworks"]),
                    (configures(CompilerPass::Linking), vec!["-framework", "Foundation"]),
                    (configures(CompilerPass::Compiling), vec!["-load", "/path/to/plugin.so"]),
                    (configures(CompilerPass::Compiling), vec!["-plugin", "my-plugin"]),
                    (Source { binary: false }, vec!["main.m"]),
                ],
            );
        }

        #[test]
        fn analysis_and_codegen_flags() {
            let sut = CompilerInterpreter::default();
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
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Compiling), vec!["--analyze"]),
                    (configures(CompilerPass::Compiling), vec!["-Xanalyzer", "-analyzer-output=text"]),
                    (configures(CompilerPass::Compiling), vec!["-emit-llvm"]),
                    (configures(CompilerPass::Compiling), vec!["-fprofile-instr-generate"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        #[test]
        fn compilation_database_flag() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang",
                vec!["clang", "-MJ", "compile_commands.json", "main.c"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang"]),
                    (configures(CompilerPass::Preprocessing), vec!["-MJ", "compile_commands.json"]),
                    (Source { binary: false }, vec!["main.c"]),
                ],
            );
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_cpath() {
            let sut = CompilerInterpreter::default();
            let cpath = create_path_string(&["/usr/include", "/opt/include"]);
            let mut env = HashMap::new();
            env.insert("CPATH", cpath.as_str());
            let execution = create_execution_with_env(
                "clang",
                vec!["clang", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 6);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-I".to_string()));
                assert!(args.contains(&"/usr/include".to_string()));
                assert!(args.contains(&"/opt/include".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_c_include_path() {
            let sut = CompilerInterpreter::default();
            let mut env = HashMap::new();
            env.insert("C_INCLUDE_PATH", "/usr/local/include");
            let execution = create_execution_with_env(
                "clang",
                vec!["clang", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 5);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-isystem".to_string()));
                assert!(args.contains(&"/usr/local/include".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_cplus_include_path() {
            let sut = CompilerInterpreter::default();
            let mut env = HashMap::new();
            env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");
            let execution = create_execution_with_env(
                "clang++",
                vec!["clang++", "-c", "main.cpp", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 5);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-isystem".to_string()));
                assert!(args.contains(&"/usr/include/c++/11".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_multiple() {
            let sut = CompilerInterpreter::default();
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
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 8);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"/usr/include".to_string()));
                assert!(args.contains(&"/opt/include".to_string()));
                assert!(args.contains(&"/usr/local/include".to_string()));
                assert!(args.contains(&"/usr/include/c++/11".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn environment_variables_empty_paths() {
            let sut = CompilerInterpreter::default();
            let c_include_path = create_path_string(&["", "", "", ""]);
            let mut env = HashMap::new();
            env.insert("CPATH", "");
            env.insert("C_INCLUDE_PATH", c_include_path.as_str());
            let execution = create_execution_with_env(
                "clang",
                vec!["clang", "-c", "main.c", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 4);
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_objc_include_path() {
            let sut = CompilerInterpreter::default();
            let objc_include_path = create_path_string(&["/System/Library/Frameworks", "/usr/local/objc"]);
            let mut env = HashMap::new();
            env.insert("OBJC_INCLUDE_PATH", objc_include_path.as_str());
            let execution = create_execution_with_env(
                "clang",
                vec!["clang", "-c", "main.m", "-o", "main.o"],
                "/project",
                env,
            );
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 6);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"-isystem".to_string()));
                assert!(args.contains(&"/System/Library/Frameworks".to_string()));
                assert!(args.contains(&"/usr/local/objc".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        /// Uses contains-checks instead of assert_command because environment
        /// variable processing order may not match HashMap iteration order.
        #[test]
        fn environment_variables_all_types() {
            let sut = CompilerInterpreter::default();
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
            let result = sut.recognize(execution);
            if let RecognizeResult::Recognized(cmd) = result {
                assert_eq!(cmd.arguments.len(), 8);
                let args: Vec<String> =
                    cmd.arguments.iter().flat_map(|a| a.as_arguments(&|p| Cow::Borrowed(p))).collect();
                assert!(args.contains(&"/usr/include".to_string()));
                assert!(args.contains(&"/usr/local/include".to_string()));
                assert!(args.contains(&"/usr/include/c++/11".to_string()));
                assert!(args.contains(&"/System/Library/Frameworks".to_string()));
                assert!(args.contains(&"-I".to_string()));
                assert!(args.contains(&"-isystem".to_string()));
            } else {
                panic!("Expected compiler command");
            }
        }

        #[test]
        fn cc1_invocation_ignored() {
            let sut = CompilerInterpreter::default();

            // User-facing clang command should be recognized
            let user_execution = create_execution(
                "clang++",
                vec!["clang++", "-c", "-std=c++23", "-o", "hello-world", "hello-world.cpp"],
                "/home/user/project",
            );
            if let RecognizeResult::Recognized(cmd) = sut.recognize(user_execution) {
                assert_eq!(cmd.arguments.len(), 5);
                assert_eq!(cmd.arguments[0].kind(), Compiler);
            } else {
                panic!("Expected compiler command for user-facing invocation");
            }

            // Internal -cc1 clang command should be ignored
            let cc1_execution = create_execution(
                "clang++",
                vec![
                    "clang++",
                    "-cc1",
                    "-triple",
                    "x86_64-pc-linux-gnu",
                    "-emit-obj",
                    "-dumpdir",
                    "hello-world-",
                    "-disable-free",
                    "-clear-ast-before-backend",
                    "-disable-llvm-verifier",
                    "-discard-value-names",
                    "-main-file-name",
                    "hello-world.cpp",
                    "-mrelocation-model",
                    "pic",
                    "-pic-level",
                    "2",
                    "-pic-is-pie",
                    "-mframe-pointer=all",
                    "-fmath-errno",
                    "-ffp-contract=on",
                    "-fno-rounding-math",
                    "-mconstructor-aliases",
                    "-funwind-tables=2",
                    "-target-cpu",
                    "x86-64",
                    "-tune-cpu",
                    "generic",
                    "-debugger-tuning=gdb",
                    "-fdebug-compilation-dir=/home/user/project",
                    "-fcoverage-compilation-dir=/home/user/project",
                    "-resource-dir",
                    "/usr/lib/clang/20",
                    "-std=c++23",
                    "-fdeprecated-macro",
                    "-ferror-limit",
                    "19",
                    "-stack-protector",
                    "2",
                    "-fgnuc-version=4.2.1",
                    "-fno-implicit-modules",
                    "-fskip-odr-check-in-gmf",
                    "-fcxx-exceptions",
                    "-fexceptions",
                    "-fcolor-diagnostics",
                    "-faddrsig",
                    "-D__GCC_HAVE_DWARF2_CFI_ASM=1",
                    "-x",
                    "c++",
                    "-o",
                    "/tmp/hello-world-bd186e.o",
                    "hello-world.cpp",
                ],
                "/home/user/project",
            );
            assert_ignored(sut.recognize(cc1_execution), "internal invocation");
        }

        // Requirements: recognition-cpp20-modules
        #[test]
        fn precompile_module_interface_does_not_source_the_output() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang++",
                vec!["clang++", "--precompile", "-std=c++20", "foo.cppm", "-o", "foo.pcm"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang++"]),
                    (stops_at(CompilerPass::Compiling), vec!["--precompile"]),
                    (configures(CompilerPass::Compiling), vec!["-std=c++20"]),
                    (Source { binary: false }, vec!["foo.cppm"]),
                    (Output, vec!["-o", "foo.pcm"]),
                ],
            );
        }

        // Requirements: recognition-cpp20-modules
        #[test]
        fn module_file_flag_consumes_precompiled_module_without_sourcing_it() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "clang++",
                vec!["clang++", "-std=c++20", "-fmodule-file=foo=foo.pcm", "-c", "main.cpp"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["clang++"]),
                    (configures(CompilerPass::Compiling), vec!["-std=c++20"]),
                    (configures(CompilerPass::Compiling), vec!["-fmodule-file=foo=foo.pcm"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (Source { binary: false }, vec!["main.cpp"]),
                ],
            );
        }
    }

    mod flang {
        use super::*;

        #[test]
        fn basic() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "flang",
                vec!["flang", "-fbackslash", "-ffree-form", "-J/path/to/modules", "-cpp", "main.f90"],
                "/project",
            );
            if let RecognizeResult::Recognized(cmd) = sut.recognize(execution) {
                assert_eq!(cmd.arguments.len(), 6);
                assert_eq!(cmd.arguments[1].kind(), configures(CompilerPass::Compiling));
                assert_eq!(cmd.arguments[2].kind(), configures(CompilerPass::Compiling));
                assert_eq!(cmd.arguments[3].kind(), configures(CompilerPass::Compiling));
                assert_eq!(cmd.arguments[4].kind(), configures(CompilerPass::Preprocessing));
            } else {
                panic!("Expected compiler command for Flang");
            }
        }
    }

    mod cuda {
        use super::*;

        #[test]
        fn basic_compilation() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("nvcc", vec!["nvcc", "-c", "kernel.cu", "-o", "kernel.o"], "/test");
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["nvcc"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (Source { binary: false }, vec!["kernel.cu"]),
                    (Output, vec!["-o", "kernel.o"]),
                ],
            );
        }

        #[test]
        fn gpu_architecture_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "nvcc",
                vec![
                    "nvcc",
                    "--gpu-architecture=sm_80",
                    "-arch=sm_70",
                    "--gpu-code=sm_80,compute_80",
                    "-c",
                    "kernel.cu",
                ],
                "/test",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["nvcc"]),
                    (configures(CompilerPass::Compiling), vec!["--gpu-architecture=sm_80"]),
                    (configures(CompilerPass::Compiling), vec!["-arch=sm_70"]),
                    (configures(CompilerPass::Compiling), vec!["--gpu-code=sm_80,compute_80"]),
                    (stops_at(CompilerPass::Compiling), vec!["-c"]),
                    (Source { binary: false }, vec!["kernel.cu"]),
                ],
            );
        }

        #[test]
        fn device_compilation_modes() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "nvcc",
                vec!["nvcc", "--device-c", "--relocatable-device-code=true", "kernel.cu", "-o", "kernel.o"],
                "/test",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["nvcc"]),
                    (stops_at(CompilerPass::Compiling), vec!["--device-c"]),
                    (configures(CompilerPass::Compiling), vec!["--relocatable-device-code=true"]),
                    (Source { binary: false }, vec!["kernel.cu"]),
                    (Output, vec!["-o", "kernel.o"]),
                ],
            );
        }

        #[test]
        fn host_compiler_passthrough() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "nvcc",
                vec!["nvcc", "-Xcompiler", "-Wall", "-Xlinker", "-rpath=/usr/lib", "main.cu"],
                "/test",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["nvcc"]),
                    (configures(CompilerPass::Compiling), vec!["-Xcompiler"]),
                    (none(), vec!["-Wall"]),
                    (configures(CompilerPass::Linking), vec!["-Xlinker"]),
                    (none(), vec!["-rpath=/usr/lib"]),
                    (Source { binary: false }, vec!["main.cu"]),
                ],
            );
        }

        #[test]
        fn debug_and_optimization() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "nvcc",
                vec!["nvcc", "-G", "--generate-line-info", "-O2", "--use_fast_math", "kernel.cu"],
                "/test",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["nvcc"]),
                    (configures(CompilerPass::Compiling), vec!["-G"]),
                    (configures(CompilerPass::Compiling), vec!["--generate-line-info"]),
                    (configures(CompilerPass::Compiling), vec!["-O2"]),
                    (configures(CompilerPass::Compiling), vec!["--use_fast_math"]),
                    (Source { binary: false }, vec!["kernel.cu"]),
                ],
            );
        }

        #[test]
        fn flag_formats() {
            let sut = CompilerInterpreter::default();

            for args in [
                vec!["nvcc", "--gpu-architecture=sm_80", "-c", "kernel.cu"],
                vec!["nvcc", "--gpu-architecture", "sm_80", "-c", "kernel.cu"],
            ] {
                let execution = create_execution("nvcc", args, "/test");
                let result = sut.recognize(execution);
                if let RecognizeResult::Recognized(cmd) = result {
                    assert_eq!(cmd.arguments.len(), 4);
                    assert_eq!(cmd.arguments[1].kind(), configures(CompilerPass::Compiling));
                }
            }
        }

        #[test]
        fn specific_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("nvcc", vec!["nvcc", "--compile", "kernel.cu"], "/test");
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["nvcc"]),
                    (stops_at(CompilerPass::Compiling), vec!["--compile"]),
                    (Source { binary: false }, vec!["kernel.cu"]),
                ],
            );
        }
    }

    mod intel_fortran {
        use super::*;

        #[test]
        fn basic_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("ifort", vec!["ifort", "-c", "test.f90"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 3);
                assert_eq!(parsed.arguments[1].kind(), stops_at(CompilerPass::Compiling));
            }
        }

        #[test]
        fn preprocessing_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "ifort",
                vec!["ifort", "-fpp", "-DDEBUG", "-I/usr/include", "test.f90"],
                "/project",
            );
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), configures(CompilerPass::Preprocessing));
                assert_eq!(parsed.arguments[2].kind(), configures(CompilerPass::Preprocessing));
            }
        }

        #[test]
        fn linking_flags() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("ifort", vec!["ifort", "-shared-intel", "-lm", "test.o"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), configures(CompilerPass::Linking));
                assert_eq!(parsed.arguments[2].kind(), configures(CompilerPass::Linking));
            }
        }

        #[test]
        fn info_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("ifort", vec!["ifort", "--version"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), info());
            }
        }
    }

    mod cray_fortran {
        use super::*;

        #[test]
        fn basic_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("crayftn", vec!["crayftn", "-c", "test.f90"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 3);
                assert_eq!(parsed.arguments[1].kind(), stops_at(CompilerPass::Compiling));
            }
        }

        #[test]
        fn preprocessing_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "crayftn",
                vec!["crayftn", "-DDEBUG", "-I/usr/include", "test.f90"],
                "/project",
            );
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), configures(CompilerPass::Preprocessing));
                assert_eq!(parsed.arguments[2].kind(), configures(CompilerPass::Preprocessing));
            }
        }

        #[test]
        fn linking_flags() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("crayftn", vec!["crayftn", "-add-rpath", "-lm", "test.o"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), configures(CompilerPass::Linking));
                assert_eq!(parsed.arguments[2].kind(), configures(CompilerPass::Linking));
            }
        }

        #[test]
        fn cray_specific_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "crayftn",
                vec!["crayftn", "-craylibs", "-target-cpu=x86_64", "test.f90"],
                "/project",
            );
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 4);
                assert_eq!(parsed.arguments[1].kind(), none());
                assert_eq!(parsed.arguments[2].kind(), none());
            }
        }

        #[test]
        fn openmp_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("crayftn", vec!["crayftn", "-openmp", "test.f90"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 3);
                assert_eq!(parsed.arguments[1].kind(), none());
            }
        }
    }

    mod mpi {
        use super::*;

        // Requirements: recognition-compiler-names
        #[test]
        fn basic_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("mpicc", vec!["mpicc", "-c", "hello.c"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 3);
                assert_eq!(parsed.arguments[1].kind(), stops_at(CompilerPass::Compiling));
                assert_eq!(parsed.arguments[2].kind(), Source { binary: false });
            }
        }

        /// Watch-out from the requirement: the glued form `-cc=gcc` must stay
        /// a single token, and must not swallow the source file that follows.
        // Requirements: recognition-compiler-names
        #[test]
        fn compiler_override_glued_form_keeps_single_token_and_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("mpicc", vec!["mpicc", "-cc=gcc", "-c", "hello.c"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 4);
                assert_eq!(parsed.arguments[1].kind(), driver());
                assert_eq!(
                    parsed.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-cc=gcc".to_string()],
                    "-cc=gcc must be retained as a single token"
                );
                assert_eq!(parsed.arguments[2].kind(), stops_at(CompilerPass::Compiling));
                assert_eq!(parsed.arguments[3].kind(), Source { binary: false });
                assert_eq!(
                    parsed.arguments[3].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["hello.c".to_string()]
                );
            }
        }

        /// The separate-token spelling (`-cc gcc`) must consume the value too,
        /// or "gcc" would be misread as a phantom source file.
        // Requirements: recognition-compiler-names
        #[test]
        fn compiler_override_separate_form_consumes_value() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("mpicc", vec!["mpicc", "-cc", "gcc", "-c", "hello.c"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                let source_count =
                    parsed.arguments.iter().filter(|a| matches!(a.kind(), Source { .. })).count();
                assert_eq!(source_count, 1, "only hello.c must be a source, got {:?}", parsed.arguments);
                assert_eq!(parsed.arguments[1].kind(), driver());
                assert_eq!(
                    parsed.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-cc".to_string(), "gcc".to_string()]
                );
            }
        }

        /// Wrapper-info invocations classify as info-and-exit, which the
        /// output converter uses to skip emitting a database entry.
        // Requirements: recognition-compiler-names
        #[test]
        fn wrapper_info_flags_are_info_and_exit() {
            let sut = CompilerInterpreter::default();
            for args in [vec!["mpicc", "-showme"], vec!["mpicc", "-show"], vec!["mpicc", "-compile_info"]] {
                let execution = create_execution("mpicc", args.clone(), "/project");
                let result = sut.recognize(execution);
                assert!(matches!(result, RecognizeResult::Recognized(_)), "args: {:?}", args);
                if let RecognizeResult::Recognized(parsed) = result {
                    assert_eq!(parsed.arguments[1].kind(), info(), "args: {:?}", args);
                }
            }
        }
    }

    mod qnx {
        use super::*;

        // Requirements: recognition-compiler-names
        #[test]
        fn basic_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("qcc", vec!["qcc", "-c", "hello.c"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 3);
                assert_eq!(parsed.arguments[1].kind(), stops_at(CompilerPass::Compiling));
                assert_eq!(parsed.arguments[2].kind(), Source { binary: false });
            }
        }

        /// QNX's variant selector is attached-value only (`-Vgcc_ntoaarch64le`);
        /// a bare `-V` (no attached value) lists variants. Both spellings are
        /// modeled as a prefix pattern with 0 extra args, so the token is never
        /// split and never swallows a following source file.
        // Requirements: recognition-compiler-names
        #[test]
        fn variant_selector_is_retained_and_does_not_swallow_source() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("qcc", vec!["qcc", "-Vgcc_ntoaarch64le", "-c", "hello.c"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 4);
                assert_eq!(parsed.arguments[1].kind(), driver());
                assert_eq!(
                    parsed.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-Vgcc_ntoaarch64le".to_string()],
                    "-Vgcc_ntoaarch64le must be retained as a single token"
                );
                assert_eq!(parsed.arguments[2].kind(), stops_at(CompilerPass::Compiling));
                assert_eq!(parsed.arguments[3].kind(), Source { binary: false });
                assert_eq!(
                    parsed.arguments[3].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["hello.c".to_string()]
                );
            }
        }

        /// A bare `-V` (no attached value) must also be treated as a driver
        /// option, never as a source file.
        // Requirements: recognition-compiler-names
        #[test]
        fn bare_variant_listing_flag_is_a_driver_option() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("qcc", vec!["qcc", "-V", "-c", "hello.c"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), driver());
                let source_count =
                    parsed.arguments.iter().filter(|a| matches!(a.kind(), Source { .. })).count();
                assert_eq!(source_count, 1, "only hello.c must be a source, got {:?}", parsed.arguments);
            }
        }
    }

    mod nasm {
        use super::*;

        /// The separate-token form of `-f` (output format) must consume its
        /// value so it is never mis-classified as a second source file, and
        /// the `-o` output pair must not swallow the assembly source that
        /// precedes it.
        // Requirements: recognition-compiler-names
        #[test]
        fn separate_format_value_is_consumed_not_classified_as_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "nasm",
                vec!["nasm", "-f", "elf64", "hello.asm", "-o", "hello.o"],
                "/project",
            );
            let result = sut.recognize(execution);
            assert_command(
                result,
                vec![
                    (ArgumentKind::Compiler, vec!["nasm"]),
                    (configures(CompilerPass::Compiling), vec!["-f", "elf64"]),
                    (Source { binary: false }, vec!["hello.asm"]),
                    (ArgumentKind::Output, vec!["-o", "hello.o"]),
                ],
            );
        }

        /// NASM's canonical lowercase `-d` define must consume its value even
        /// when that value ends in a source extension (`-d NAME=release.asm`
        /// parameterizes a %include); leaking it into source detection would
        /// fabricate a second compilation entry.
        // Requirements: recognition-compiler-names
        #[test]
        fn lowercase_define_value_with_source_extension_is_not_a_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "nasm",
                vec!["nasm", "-d", "CONFIG=release.asm", "-f", "elf64", "hello.asm"],
                "/project",
            );
            let result = sut.recognize(execution);
            assert_command(
                result,
                vec![
                    (ArgumentKind::Compiler, vec!["nasm"]),
                    (configures(CompilerPass::Preprocessing), vec!["-d", "CONFIG=release.asm"]),
                    (configures(CompilerPass::Compiling), vec!["-f", "elf64"]),
                    (Source { binary: false }, vec!["hello.asm"]),
                ],
            );
        }

        /// The glued form (`-felf64`) must be recognized the same way as the
        /// separate-token form.
        // Requirements: recognition-compiler-names
        #[test]
        fn glued_format_value_is_consumed_not_classified_as_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("nasm", vec!["nasm", "-felf64", "hello.asm"], "/project");
            let result = sut.recognize(execution);
            assert_command(
                result,
                vec![
                    (ArgumentKind::Compiler, vec!["nasm"]),
                    (configures(CompilerPass::Compiling), vec!["-felf64"]),
                    (Source { binary: false }, vec!["hello.asm"]),
                ],
            );
        }

        /// `nasm -v` prints version info and exits; there is no source
        /// argument, so no compilation entry can be synthesized from it.
        // Requirements: recognition-compiler-names
        #[test]
        fn version_flag_has_no_source_argument() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("nasm", vec!["nasm", "-v"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments[1].kind(), info());
                assert!(
                    !parsed.arguments.iter().any(|a| matches!(a.kind(), Source { .. })),
                    "got {:?}",
                    parsed.arguments
                );
            }
        }

        /// YASM's long `--version` option is covered by the `--*` catch-all
        /// (no separate-token value ever follows a YASM long option), and
        /// still has no source argument to build an entry from.
        // Requirements: recognition-compiler-names
        #[test]
        fn yasm_long_version_option_has_no_source_argument() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("yasm", vec!["yasm", "--version"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert!(
                    !parsed.arguments.iter().any(|a| matches!(a.kind(), Source { .. })),
                    "got {:?}",
                    parsed.arguments
                );
            }
        }
    }

    mod fasm {
        use super::*;

        /// `-m`'s memory-limit value is always a separate token; it must be
        /// consumed so it is never mistaken for a source file, and the
        /// trailing output positional (`hello.o`, not a recognized source
        /// extension) must not turn into a second compilation entry.
        // Requirements: recognition-compiler-names
        #[test]
        fn memory_limit_value_is_consumed_and_output_positional_is_not_a_source() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("fasm", vec!["fasm", "-m", "65536", "hello.asm", "hello.o"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 4);
                assert_eq!(parsed.arguments[1].kind(), none());
                assert_eq!(
                    parsed.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-m".to_string(), "65536".to_string()]
                );
                assert_eq!(parsed.arguments[2].kind(), Source { binary: false });
                assert_eq!(parsed.arguments[3].kind(), Source { binary: true }, "got {:?}", parsed.arguments);
            }
        }
    }

    mod swift {
        use super::*;

        // Requirements: recognition-compiler-names
        #[test]
        fn basic_compilation() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution("swiftc", vec!["swiftc", "-c", "hello.swift"], "/project");
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                assert_eq!(parsed.arguments.len(), 3);
                assert_eq!(parsed.arguments[1].kind(), stops_at(CompilerPass::Compiling));
                assert_eq!(parsed.arguments[2].kind(), Source { binary: false });
            }
        }

        /// The motivating whole-module shape: several `.swift` sources in one
        /// invocation. Recognition/parsing must classify both as compilable
        /// sources and consume `-module-name`'s separate-token value without
        /// treating it (or "App") as a source; the converter (tested
        /// separately) is what fans this into one entry per source, each
        /// keeping every token.
        // Requirements: recognition-compiler-names
        #[test]
        fn whole_module_invocation_keeps_all_sources_and_flags() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "swiftc",
                vec!["swiftc", "-module-name", "App", "-emit-object", "a.swift", "b.swift"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["swiftc"]),
                    (configures(CompilerPass::Compiling), vec!["-module-name", "App"]),
                    (stops_at(CompilerPass::Compiling), vec!["-emit-object"]),
                    (Source { binary: false }, vec!["a.swift"]),
                    (Source { binary: false }, vec!["b.swift"]),
                ],
            );
        }

        /// swiftc spawns per-file `swift-frontend` jobs the way gcc spawns
        /// `cc1`; the internal executable name is filtered via `ignore_when`.
        // Requirements: recognition-compiler-names
        #[test]
        fn swift_frontend_executable_is_ignored() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "swift-frontend",
                vec!["swift-frontend", "-frontend", "-c", "a.swift"],
                "/project",
            );
            assert_ignored(sut.recognize(execution), "internal executable");
        }

        /// A legacy toolchain re-invoking itself as `swiftc -frontend` must
        /// also be filtered, via the `ignore_when.flags` list.
        // Requirements: recognition-compiler-names
        #[test]
        fn frontend_flag_execution_is_ignored() {
            let sut = CompilerInterpreter::default();
            let execution =
                create_execution("swiftc", vec!["swiftc", "-frontend", "-c", "a.swift"], "/project");
            assert_ignored(sut.recognize(execution), "internal invocation");
        }

        /// `swiftc --version`/`-version` print info and exit; there is no
        /// source argument, so no compilation entry can be synthesized.
        // Requirements: recognition-compiler-names
        #[test]
        fn version_flags_are_info_and_exit() {
            let sut = CompilerInterpreter::default();
            for args in [vec!["swiftc", "--version"], vec!["swiftc", "-version"]] {
                let execution = create_execution("swiftc", args.clone(), "/project");
                let result = sut.recognize(execution);
                assert!(matches!(result, RecognizeResult::Recognized(_)), "args: {:?}", args);
                if let RecognizeResult::Recognized(parsed) = result {
                    assert_eq!(parsed.arguments[1].kind(), info(), "args: {:?}", args);
                    assert!(
                        !parsed.arguments.iter().any(|a| matches!(a.kind(), Source { .. })),
                        "got {:?}",
                        parsed.arguments
                    );
                }
            }
        }

        /// `-Xcc`/`-Xlinker`/`-Xfrontend` each forward exactly one separate
        /// token to a downstream tool; that token often starts with '-'
        /// (`-Xlinker -rpath`), so `count: 1` must consume it unconditionally
        /// or it would leak in as a phantom source.
        // Requirements: recognition-compiler-names
        #[test]
        fn forwarded_flags_consume_dash_prefixed_value_not_a_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "swiftc",
                vec![
                    "swiftc",
                    "-Xlinker",
                    "-rpath",
                    "-Xcc",
                    "-DFOO",
                    "-Xfrontend",
                    "-enable-cross-import-overlays",
                    "a.swift",
                ],
                "/project",
            );
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                let source_count =
                    parsed.arguments.iter().filter(|a| matches!(a.kind(), Source { .. })).count();
                assert_eq!(source_count, 1, "only a.swift must be a source, got {:?}", parsed.arguments);
                assert_eq!(parsed.arguments[1].kind(), configures(CompilerPass::Linking));
                assert_eq!(
                    parsed.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-Xlinker".to_string(), "-rpath".to_string()]
                );
                assert_eq!(parsed.arguments[2].kind(), configures(CompilerPass::Compiling));
                assert_eq!(
                    parsed.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-Xcc".to_string(), "-DFOO".to_string()]
                );
                assert_eq!(parsed.arguments[3].kind(), configures(CompilerPass::Compiling));
                assert_eq!(
                    parsed.arguments[3].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-Xfrontend".to_string(), "-enable-cross-import-overlays".to_string()]
                );
            }
        }

        /// `-D`/`-I` are JoinedOrSeparate in swiftc (unlike the Separate-only
        /// `-target`/`-module-name`); both the glued and separate spellings
        /// must be recognized so their value never leaks in as a source.
        // Requirements: recognition-compiler-names
        #[test]
        fn define_and_import_path_accept_glued_or_separate_value() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "swiftc",
                vec!["swiftc", "-DDEBUG", "-I", "/usr/local/include", "a.swift"],
                "/project",
            );
            assert_command(
                sut.recognize(execution),
                vec![
                    (Compiler, vec!["swiftc"]),
                    (configures(CompilerPass::Preprocessing), vec!["-DDEBUG"]),
                    (configures(CompilerPass::Preprocessing), vec!["-I", "/usr/local/include"]),
                    (Source { binary: false }, vec!["a.swift"]),
                ],
            );
        }

        /// `-import-objc-header`'s value is a `.h` path -- and `.h` IS a
        /// recognized source extension for other families (headers are
        /// translation units elsewhere), so an unmatched (0-arg) rule here
        /// would leak the bridging header in as a phantom compilable
        /// source, corrupting entry count. `count: 1` must consume it.
        // Requirements: recognition-compiler-names
        #[test]
        fn bridging_header_value_is_consumed_not_a_phantom_source() {
            let sut = CompilerInterpreter::default();
            let execution = create_execution(
                "swiftc",
                vec!["swiftc", "-import-objc-header", "Bridging.h", "-module-name", "App", "a.swift"],
                "/project",
            );
            let result = sut.recognize(execution);
            assert!(matches!(result, RecognizeResult::Recognized(_)));
            if let RecognizeResult::Recognized(parsed) = result {
                let source_count =
                    parsed.arguments.iter().filter(|a| matches!(a.kind(), Source { .. })).count();
                assert_eq!(source_count, 1, "only a.swift must be a source, got {:?}", parsed.arguments);
                assert_eq!(parsed.arguments[1].kind(), configures(CompilerPass::Compiling));
                assert_eq!(
                    parsed.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                    vec!["-import-objc-header".to_string(), "Bridging.h".to_string()]
                );
            }
        }
    }
}

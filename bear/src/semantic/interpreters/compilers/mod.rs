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
        let mut result = Self::new(Arc::clone(&recognizer));

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
    use crate::semantic::{ArgumentKind, CompilerPass, PassEffect};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

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

    // -----------------------------------------------------------------------
    // Existing mod.rs tests (black-box, kept as-is)
    // -----------------------------------------------------------------------

    #[test]
    fn test_gcc_recognition_and_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("/usr/bin/gcc", vec!["/usr/bin/gcc", "-c", "test.c"], "/tmp");

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
        let execution = create_execution("clang", vec!["clang", "-c", "main.c", "-o", "main.o"], "/tmp");

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
        let execution = create_execution("unknown_compiler", vec!["unknown_compiler", "-c", "test.c"], "/tmp");

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
            arguments: vec!["gcc".to_string(), "-c".to_string(), "file.c".to_string()],
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
        let custom_gcc = create_execution("/custom/path/my-gcc", vec!["/custom/path/my-gcc", "-c", "test.c"], "/tmp");
        let result = sut.recognize(&custom_gcc);
        assert!(result.is_some(), "Custom GCC path should be recognized via config hint");

        // Test custom Clang path
        let custom_clang = create_execution("/opt/clang/bin/clang++", vec!["/opt/clang/bin/clang++", "-c", "main.cpp"], "/tmp");
        let result = sut.recognize(&custom_clang);
        assert!(result.is_some(), "Custom Clang path should be recognized via config hint");

        // Test that normal compiler paths still work
        let normal_gcc = create_execution("gcc", vec!["gcc", "-c", "normal.c"], "/tmp");
        let result = sut.recognize(&normal_gcc);
        assert!(result.is_some(), "Standard GCC should still be recognized");
    }

    #[test]
    fn test_wrapper_recognition_and_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test ccache wrapper
        let ccache_execution = create_execution("ccache", vec!["ccache", "gcc", "-c", "test.c"], "/tmp");
        let result = sut.recognize(&ccache_execution);

        if let Some(Command::Compiler(cmd)) = result {
            // Assert the compiler
            assert_eq!(*cmd.executable, *"gcc");
            // Assert the arguments
            let arguments: Vec<String> =
                cmd.arguments.into_iter().flat_map(|arg| arg.as_arguments(&noop)).collect();
            assert_eq!(vec!["gcc".to_string(), "-c".to_string(), "test.c".to_string()], arguments);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_uniform_delegation() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test that all compiler types are handled uniformly through the map
        let executables = vec!["gcc", "clang", "nvcc", "gfortran", "ifort"];
        for executable in executables {
            let execution = create_execution(executable, vec![executable, "-c", "test.c"], "/tmp");

            // Test that the recognizer identifies the correct type
            let recognized_type = sut.recognizer.recognize(&execution.executable);
            if let Some(compiler_type) = recognized_type {
                // If it's recognized, it should delegate properly through the map
                let result = sut.interpreters.get(&compiler_type);
                assert!(result.is_some(), "Interpreter should be registered for {}", executable);
            }
        }
    }

    // -----------------------------------------------------------------------
    // GCC tests (migrated from gcc.rs)
    // -----------------------------------------------------------------------

    #[test]
    fn test_gcc_simple_compilation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project");

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: false });
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_combined_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-I/usr/include", "-DDEBUG=1", "-o", "main", "main.c"],
            "/project",
        );

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 5);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["-I/usr/include"]);

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)), vec!["-DDEBUG=1"]);

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
            assert_eq!(cmd.arguments[3].as_arguments(&|p| Cow::Borrowed(p)), vec!["-o", "main"]);

            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_separate_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution =
            create_execution("gcc", vec!["gcc", "-I", "/usr/include", "-D", "DEBUG=1", "main.c"], "/project");

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["-I", "/usr/include"]);

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)), vec!["-D", "DEBUG=1"]);

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_response_file() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("gcc", vec!["gcc", "@response.txt", "main.c"], "/project");

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 3);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["@response.txt"]);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_warning_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution =
            create_execution("gcc", vec!["gcc", "-Wall", "-Wextra", "-Wno-unused", "main.c"], "/project");

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 5);

            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(PassEffect::None));
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["-Wall"]);

            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Other(PassEffect::None));

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Other(PassEffect::None));
            assert_eq!(cmd.arguments[3].as_arguments(&|p| Cow::Borrowed(p)), vec!["-Wno-unused"]);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_std_flag_variations() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test separate form: -std c99
        let execution = create_execution("gcc", vec!["gcc", "-std", "c99", "main.c"], "/project");
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["-std", "c99"]);
        }

        // Test equals form: -std=c99
        let execution = create_execution("gcc", vec!["gcc", "-std=c99", "main.c"], "/project");
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["-std=c99"]);
        }
    }

    #[test]
    fn test_gcc_complex_compilation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
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

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert!(cmd.arguments.len() >= 10);

            let mut source_count = 0;
            let mut output_count = 0;

            for arg in &cmd.arguments {
                match arg.kind() {
                    ArgumentKind::Source { .. } => source_count += 1,
                    ArgumentKind::Output => output_count += 1,
                    _ => {}
                }
            }

            assert_eq!(source_count, 2);
            assert_eq!(output_count, 1);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_comprehensive_flag_coverage() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test optimization flags with prefix matching
        let execution =
            create_execution("gcc", vec!["gcc", "-O2", "-Os", "-Ofast", "-Og", "main.c"], "/project");
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert!(cmd.arguments.len() >= 5);
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }
        }

        // Test debug flags with prefix matching
        let execution =
            create_execution("gcc", vec!["gcc", "-g", "-g3", "-gdwarf-4", "-ggdb", "main.c"], "/project");
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }
        }

        // Test warning flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-Wall", "-Wextra", "-Wno-unused", "-Werror=format", "main.c"],
            "/project",
        );
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            for i in 1..5 {
                assert_eq!(cmd.arguments[i].kind(), ArgumentKind::Other(PassEffect::None));
            }
        }

        // Test feature flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-fPIC", "-fstack-protector", "-fno-omit-frame-pointer", "-flto", "main.c"],
            "/project",
        );
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }
        }

        // Test machine flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-m64", "-march=native", "-mtune=generic", "-msse4.2", "main.c"],
            "/project",
        );
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }
        }
    }

    #[test]
    fn test_gcc_linker_and_system_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
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
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
        }
    }

    #[test]
    fn test_gcc_response_files_and_plugins() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Other(PassEffect::DriverOption));
        }
    }

    #[test]
    fn test_gcc_environment_variables_cpath() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let cpath = create_path_string(&["/usr/include", "/opt/include"]);
        let mut env = HashMap::new();
        env.insert("CPATH", cpath.as_str());

        let execution =
            create_execution_with_env("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 6);

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
    fn test_gcc_environment_variables_c_include_path() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let mut env = HashMap::new();
        env.insert("C_INCLUDE_PATH", "/usr/local/include");

        let execution =
            create_execution_with_env("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
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
    fn test_gcc_environment_variables_cplus_include_path() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let mut env = HashMap::new();
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");

        let execution =
            create_execution_with_env("g++", vec!["g++", "-c", "main.cpp", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
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
    fn test_gcc_environment_variables_multiple() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let cpath = create_path_string(&["/usr/include", "/opt/include"]);
        let mut env = HashMap::new();
        env.insert("CPATH", cpath.as_str());
        env.insert("C_INCLUDE_PATH", "/usr/local/include");
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");

        let execution =
            create_execution_with_env("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 8);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            assert!(args_as_strings.contains(&"/usr/include".to_string()));
            assert!(args_as_strings.contains(&"/opt/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/local/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/include/c++/11".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_environment_variables_objc_include_path() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let objc_include_path = create_path_string(&["/System/Library/Frameworks", "/usr/local/objc"]);
        let mut env = HashMap::new();
        env.insert("OBJC_INCLUDE_PATH", objc_include_path.as_str());

        let execution =
            create_execution_with_env("gcc", vec!["gcc", "-c", "main.m", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
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
    fn test_gcc_environment_variables_all_types() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let mut env = HashMap::new();
        env.insert("CPATH", "/usr/include");
        env.insert("C_INCLUDE_PATH", "/usr/local/include");
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");
        env.insert("OBJC_INCLUDE_PATH", "/System/Library/Frameworks");

        let execution =
            create_execution_with_env("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 8);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

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

    #[test]
    fn test_gcc_environment_variables_empty_paths() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let c_include_path = create_path_string(&["", "", "", ""]);
        let mut env = HashMap::new();
        env.insert("CPATH", "");
        env.insert("C_INCLUDE_PATH", c_include_path.as_str());

        let execution =
            create_execution_with_env("gcc", vec!["gcc", "-c", "main.c", "-o", "main.o"], "/project", env);

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_preprocessor_comprehensive_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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
        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing))
            );
            for i in 2..13 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
                );
            }
        }
    }

    #[test]
    fn test_gcc_internal_executables_are_ignored() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test cc1 (C compiler proper)
        let cc1_execution = create_execution(
            "/usr/libexec/gcc/x86_64-linux-gnu/11/cc1",
            vec!["cc1", "-quiet", "test.c"],
            "/home/user",
        );

        if let Some(Command::Ignored(reason)) = sut.recognize(&cc1_execution) {
            assert_eq!(reason, "GCC internal executable");
        } else {
            panic!("Expected ignored command for cc1");
        }

        // Test cc1plus (C++ compiler proper)
        let cc1plus_execution = create_execution(
            "/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus",
            vec!["cc1plus", "-quiet", "test.cpp"],
            "/home/user",
        );

        if let Some(Command::Ignored(reason)) = sut.recognize(&cc1plus_execution) {
            assert_eq!(reason, "GCC internal executable");
        } else {
            panic!("Expected ignored command for cc1plus");
        }

        // Test collect2 (linker wrapper)
        let collect2_execution = create_execution(
            "/usr/libexec/gcc/x86_64-linux-gnu/11/collect2",
            vec!["collect2", "-o", "program", "main.o"],
            "/home/user",
        );

        if let Some(Command::Ignored(reason)) = sut.recognize(&collect2_execution) {
            assert_eq!(reason, "GCC internal executable");
        } else {
            panic!("Expected ignored command for collect2");
        }

        // Test internal fortran executable
        let fortran_execution = create_execution(
            "/usr/libexec/gcc/x86_64-redhat-linux/15/f951",
            vec!["f951", "fortran.f90", "-mtune=generic", "-march=x86-64", "-o", "/tmp/cc6kwJ3Y.s"],
            "/home/user",
        );

        if let Some(Command::Ignored(reason)) = sut.recognize(&fortran_execution) {
            assert_eq!(reason, "GCC internal executable");
        } else {
            panic!("Expected ignored command for f951");
        }

        // Test that regular gcc commands are still recognized as compilers
        let gcc_execution =
            create_execution("/usr/bin/gcc", vec!["gcc", "-c", "-O2", "main.c"], "/home/user");

        if let Some(Command::Compiler(_)) = sut.recognize(&gcc_execution) {
            // This is expected
        } else {
            panic!("Expected compiler command for regular gcc");
        }
    }

    #[test]
    fn test_gcc_linker_command_with_object_files() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution(
            "gcc",
            vec!["gcc", "-o", "a.out", "source1.o", "source2.o", "-lx", "-L/usr/local/lib"],
            "/project",
        );

        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Output);
            assert_eq!(cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)), vec!["-o", "a.out"]);

            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: true });
            assert_eq!(cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)), vec!["source1.o"]);

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: true });
            assert_eq!(cmd.arguments[3].as_arguments(&|p| Cow::Borrowed(p)), vec!["source2.o"]);

            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(cmd.arguments[4].as_arguments(&|p| Cow::Borrowed(p)), vec!["-lx"]);

            assert_eq!(
                cmd.arguments[5].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(cmd.arguments[5].as_arguments(&|p| Cow::Borrowed(p)), vec!["-L/usr/local/lib"]);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_gcc_comprehensive_linker_scenarios() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test mixed compilation and linking with various flag formats
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

        let result = sut.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert!(cmd.arguments.len() >= 10);

            let linking_args: Vec<_> = cmd
                .arguments
                .iter()
                .filter(|arg| {
                    matches!(arg.kind(), ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)))
                })
                .collect();

            assert_eq!(linking_args.len(), 7);
        }

        // Test pure linking command (no source files, only object files)
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

        let result = sut.recognize(&pure_linking).unwrap();
        if let Command::Compiler(cmd) = result {
            let object_files: Vec<_> = cmd
                .arguments
                .iter()
                .filter(|arg| {
                    let args = arg.as_arguments(&|p| Cow::Borrowed(p));
                    args.len() == 1 && (args[0].ends_with(".o") || args[0].ends_with(".a"))
                })
                .collect();

            assert_eq!(object_files.len(), 4);

            for obj_file in object_files {
                assert_eq!(obj_file.kind(), ArgumentKind::Source { binary: true });
            }
        }
    }

    #[test]
    fn test_gcc_arch_flag_preserves_argument() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution =
            create_execution("gcc", vec!["gcc", "-arch", "arm64", "-Wall", "-O2", "-c", "hello.c"], "/project");

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            let arch_arg = cmd.arguments.iter().find(|a| {
                let tokens = a.as_arguments(&|p| Cow::Borrowed(p));
                tokens.len() == 2 && tokens[0] == "-arch" && tokens[1] == "arm64"
            });
            assert!(arch_arg.is_some(), "-arch arm64 should be captured as a single argument pair");
            assert_eq!(
                arch_arg.unwrap().kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            let bad_source = cmd.arguments.iter().any(|a| {
                let tokens = a.as_arguments(&|p| Cow::Borrowed(p));
                tokens.len() == 1 && tokens[0] == "arm64"
            });
            assert!(!bad_source, "arm64 must not be misclassified as a source file");
        } else {
            panic!("Expected compiler command");
        }
    }

    // -----------------------------------------------------------------------
    // Clang tests (migrated from clang.rs)
    // -----------------------------------------------------------------------

    #[test]
    fn test_clang_simple_clang_compilation() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution("clang", vec!["clang", "-c", "-O2", "main.c"], "/project");

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_specific_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(PassEffect::None));

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["--target", "x86_64-apple-darwin"]
            );

            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_optimization_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution(
            "clang",
            vec!["clang", "-O3", "-flto", "-fsave-optimization-record", "main.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 5);

            for i in 1..4 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }

            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_target_flag_variations() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test --target form
        let execution =
            create_execution("clang", vec!["clang", "--target", "arm64-apple-macos", "main.c"], "/project");

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 3);
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["--target", "arm64-apple-macos"]
            );
        }

        // Test -target form
        let execution =
            create_execution("clang", vec!["clang", "-target", "arm64-apple-macos", "main.c"], "/project");

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 3);
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-target", "arm64-apple-macos"]
            );
        }
    }

    #[test]
    fn test_clang_sanitizer_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 5);

            for i in 1..4 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }

            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_mllvm_flag() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution(
            "clang",
            vec!["clang", "-O2", "-mllvm", "-inline-threshold=100", "myfile.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-mllvm", "-inline-threshold=100"]
            );

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_mllvm_flag_equals_form() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution(
            "clang",
            vec!["clang", "-O2", "-mllvm=-inline-threshold=100", "myfile.c"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-mllvm=-inline-threshold=100"]
            );

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_comprehensive_flag_coverage() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 13);

            for i in 1..12 {
                match cmd.arguments[i].kind() {
                    ArgumentKind::Other(PassEffect::Configures(_)) => {}
                    ArgumentKind::Other(PassEffect::StopsAt(_)) => {}
                    ArgumentKind::Other(PassEffect::None) => {}
                    other => panic!("Unexpected argument kind at index {}: {:?}", i, other),
                }
            }

            assert_eq!(cmd.arguments[12].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_cross_compilation_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_cuda_and_openmp_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 7);

            for i in 1..6 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }

            assert_eq!(cmd.arguments[6].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_framework_and_plugin_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );

            for i in 3..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }

            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_analysis_and_codegen_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

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

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_compilation_database_flag() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution =
            create_execution("clang", vec!["clang", "-MJ", "compile_commands.json", "main.c"], "/project");

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 3);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-MJ", "compile_commands.json"]
            );

            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_environment_variables_cpath() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let cpath = create_path_string(&["/usr/include", "/opt/include"]);
        let mut env = HashMap::new();
        env.insert("CPATH", cpath.as_str());

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 6);

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
    fn test_clang_environment_variables_c_include_path() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let mut env = HashMap::new();
        env.insert("C_INCLUDE_PATH", "/usr/local/include");

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.c", "-o", "main.o"],
            "/project",
            env,
        );

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
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
    fn test_clang_environment_variables_cplus_include_path() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let mut env = HashMap::new();
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");

        let execution = create_execution_with_env(
            "clang++",
            vec!["clang++", "-c", "main.cpp", "-o", "main.o"],
            "/project",
            env,
        );

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
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
    fn test_clang_environment_variables_multiple() {
        let sut = CompilerInterpreter::new_with_config(&[]);
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

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 8);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

            assert!(args_as_strings.contains(&"/usr/include".to_string()));
            assert!(args_as_strings.contains(&"/opt/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/local/include".to_string()));
            assert!(args_as_strings.contains(&"/usr/include/c++/11".to_string()));
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_environment_variables_empty_paths() {
        let sut = CompilerInterpreter::new_with_config(&[]);
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

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_clang_environment_variables_objc_include_path() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let objc_include_path = create_path_string(&["/System/Library/Frameworks", "/usr/local/objc"]);
        let mut env = HashMap::new();
        env.insert("OBJC_INCLUDE_PATH", objc_include_path.as_str());

        let execution = create_execution_with_env(
            "clang",
            vec!["clang", "-c", "main.m", "-o", "main.o"],
            "/project",
            env,
        );

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
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
    fn test_clang_environment_variables_all_types() {
        let sut = CompilerInterpreter::new_with_config(&[]);
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

        let result = sut.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 8);

            let args_as_strings: Vec<String> = cmd
                .arguments
                .iter()
                .flat_map(|arg| arg.as_arguments(&|path| std::borrow::Cow::Borrowed(path)))
                .collect();

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

    #[test]
    fn test_clang_cc1_invocation_ignored() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test the user-facing clang command (should be recognized)
        let user_execution = create_execution(
            "clang++",
            vec!["clang++", "-c", "-std=c++23", "-o", "hello-world", "hello-world.cpp"],
            "/home/user/project",
        );

        if let Some(Command::Compiler(cmd)) = sut.recognize(&user_execution) {
            assert_eq!(cmd.arguments.len(), 5);
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);
        } else {
            panic!("Expected compiler command for user-facing invocation");
        }

        // Test the internal -cc1 clang command (should be ignored)
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

        if let Some(Command::Ignored(reason)) = sut.recognize(&cc1_execution) {
            assert_eq!(reason, "clang internal invocation");
        } else {
            panic!("Expected ignored command for -cc1 invocation");
        }
    }

    #[test]
    fn test_flang_basic() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution(
            "flang",
            vec!["flang", "-fbackslash", "-ffree-form", "-J/path/to/modules", "-cpp", "main.f90"],
            "/project",
        );

        if let Some(Command::Compiler(cmd)) = sut.recognize(&execution) {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
        } else {
            panic!("Expected compiler command for Flang");
        }
    }

    // -----------------------------------------------------------------------
    // CUDA tests (migrated from cuda.rs)
    // -----------------------------------------------------------------------

    #[test]
    fn test_cuda_basic_cuda_compilation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution =
            create_execution("nvcc", vec!["nvcc", "-c", "kernel.cu", "-o", "kernel.o"], "/test");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 4);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );

            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: false });

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
            assert_eq!(
                cmd.arguments[3].as_arguments(&|p| std::borrow::Cow::Borrowed(p)),
                vec!["-o", "kernel.o"]
            );
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_gpu_architecture_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
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

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_device_compilation_modes() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution(
            "nvcc",
            vec!["nvcc", "--device-c", "--relocatable-device-code=true", "kernel.cu", "-o", "kernel.o"],
            "/test",
        );

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 5);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );

            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source { binary: false });

            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Output);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_host_compiler_passthrough() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution(
            "nvcc",
            vec!["nvcc", "-Xcompiler", "-Wall", "-Xlinker", "-rpath=/usr/lib", "main.cu"],
            "/test",
        );

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );

            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Other(PassEffect::None));

            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );

            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Other(PassEffect::None));

            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_debug_and_optimization() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution(
            "nvcc",
            vec!["nvcc", "-G", "--generate-line-info", "-O2", "--use_fast_math", "kernel.cu"],
            "/test",
        );

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 6);

            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
                );
            }

            assert_eq!(cmd.arguments[5].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_cuda_flag_formats() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        // Test equals format
        let execution = create_execution("nvcc", vec!["nvcc", "--gpu-architecture=sm_80", "-c", "kernel.cu"], "/test");
        let result = sut.recognize(&execution);
        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 4);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
        }

        // Test separate format
        let execution =
            create_execution("nvcc", vec!["nvcc", "--gpu-architecture", "sm_80", "-c", "kernel.cu"], "/test");
        let result = sut.recognize(&execution);
        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 4);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling))
            );
        }
    }

    #[test]
    fn test_cuda_specific_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);

        let execution = create_execution("nvcc", vec!["nvcc", "--compile", "kernel.cu"], "/test");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(cmd)) = result {
            assert_eq!(cmd.arguments.len(), 3);
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source { binary: false });
        } else {
            panic!("Expected compiler command");
        }
    }

    // -----------------------------------------------------------------------
    // Intel Fortran tests (migrated from intel_fortran.rs)
    // -----------------------------------------------------------------------

    #[test]
    fn test_intel_fortran_basic_compilation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("ifort", vec!["ifort", "-c", "test.f90"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 3);

            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
        }
    }

    #[test]
    fn test_intel_fortran_preprocessing_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution(
            "ifort",
            vec!["ifort", "-fpp", "-DDEBUG", "-I/usr/include", "test.f90"],
            "/project",
        );

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
        }
    }

    #[test]
    fn test_intel_fortran_linking_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution =
            create_execution("ifort", vec!["ifort", "-shared-intel", "-lm", "test.o"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
        }
    }

    #[test]
    fn test_intel_fortran_info_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("ifort", vec!["ifort", "--version"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(PassEffect::InfoAndExit));
        }
    }

    // -----------------------------------------------------------------------
    // Cray Fortran tests (migrated from cray_fortran.rs)
    // -----------------------------------------------------------------------

    #[test]
    fn test_cray_fortran_basic_compilation() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("crayftn", vec!["crayftn", "-c", "test.f90"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 3);

            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
        }
    }

    #[test]
    fn test_cray_fortran_preprocessing_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution =
            create_execution("crayftn", vec!["crayftn", "-DDEBUG", "-I/usr/include", "test.f90"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
        }
    }

    #[test]
    fn test_cray_fortran_linking_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution =
            create_execution("crayftn", vec!["crayftn", "-add-rpath", "-lm", "test.o"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
        }
    }

    #[test]
    fn test_cray_fortran_cray_specific_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution(
            "crayftn",
            vec!["crayftn", "-craylibs", "-target-cpu=x86_64", "test.f90"],
            "/project",
        );

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 4);
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(PassEffect::None));
            assert_eq!(parsed.arguments[2].kind(), ArgumentKind::Other(PassEffect::None));
        }
    }

    #[test]
    fn test_cray_fortran_openmp_flags() {
        let sut = CompilerInterpreter::new_with_config(&[]);
        let execution = create_execution("crayftn", vec!["crayftn", "-openmp", "test.f90"], "/project");

        let result = sut.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 3);
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(PassEffect::None));
        }
    }
}

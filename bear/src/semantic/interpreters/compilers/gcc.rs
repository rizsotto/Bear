// SPDX-License-Identifier: GPL-3.0-or-later

//! GCC command-line argument parser for compilation database generation.
//!
//! This module provides a specialized interpreter for parsing GCC and GCC-compatible
//! compiler command lines. It recognizes various compiler flags and categorizes them
//! into semantic groups (source files, output files, compilation options, etc.) to
//! generate accurate compilation database entries.
//!
//! The interpreter assumes compiler recognition has been handled by the compiler_interpreter
//! and focuses solely on argument parsing and semantic analysis of command-line flags.

use super::super::matchers::{looks_like_a_source_file, FlagAnalyzer, FlagPattern, FlagRule};
use super::arguments::{OtherArguments, OutputArgument, SourceArgument};
use crate::environment::{
    KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3, KEY_GCC__OBJC_INCLUDE,
};
use crate::semantic::{
    ArgumentKind, Arguments, Command, CompilerCommand, CompilerPass, Execution, Interpreter,
};

/// GCC command-line argument parser that extracts semantic information from compiler invocations.
///
/// This interpreter processes GCC and GCC-compatible compiler command lines to identify:
/// - Source files being compiled
/// - Output files and directories
/// - Compiler flags that affect compilation
/// - Include directories and preprocessor definitions
///
/// It assumes the executable has already been recognized as GCC-compatible by the
/// compiler recognition system and focuses purely on argument parsing.
pub struct GccInterpreter {
    /// Flag analyzer that recognizes and categorizes GCC command-line flags
    matcher: FlagAnalyzer,
}

impl Default for GccInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl GccInterpreter {
    /// Creates a new GCC interpreter with comprehensive GCC flag definitions.
    ///
    /// The interpreter is configured with patterns to recognize standard GCC flags
    /// including optimization flags, warning flags, include directories, preprocessor
    /// definitions, and various compilation options.
    pub fn new() -> Self {
        Self {
            matcher: FlagAnalyzer::new(&GCC_FLAGS),
        }
    }
}

impl Interpreter for GccInterpreter {
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

/// GCC flag definitions using pattern matching for argument parsing
///
/// https://gcc.gnu.org/onlinedocs/gcc/Option-Summary.html
pub static GCC_FLAGS: std::sync::LazyLock<Vec<FlagRule>> = std::sync::LazyLock::new(|| {
    let mut flags = vec![
        // Basic compilation control flags
        FlagRule::new(
            FlagPattern::Exactly("-c", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-E", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-S", 0),
            ArgumentKind::Other(Some(CompilerPass::Assembling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-r", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(FlagPattern::Exactly("-pipe", 0), ArgumentKind::Other(None)),
        FlagRule::new(
            FlagPattern::Exactly("-v", 0),
            ArgumentKind::Other(Some(CompilerPass::Info)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-###", 0),
            ArgumentKind::Other(Some(CompilerPass::Info)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("--help", 0),
            ArgumentKind::Other(Some(CompilerPass::Info)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--version", 0),
            ArgumentKind::Other(Some(CompilerPass::Info)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-ansi", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Debug information flags - using prefix for -g family
        FlagRule::new(
            FlagPattern::Prefix("-g", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Optimization flags - using prefix for comprehensive coverage
        FlagRule::new(
            FlagPattern::Prefix("-O", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Warning flags - comprehensive prefix coverage
        FlagRule::new(FlagPattern::Exactly("-w", 0), ArgumentKind::Other(None)),
        FlagRule::new(FlagPattern::Prefix("-W", 0), ArgumentKind::Other(None)),
        // Feature flags - comprehensive prefix coverage
        FlagRule::new(
            FlagPattern::Prefix("-f", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Machine/target flags - comprehensive prefix coverage
        FlagRule::new(
            FlagPattern::Prefix("-m", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Include directories and system includes
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-I"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-isystem", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-iquote", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-idirafter", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-iprefix", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-iwithprefix", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-iwithprefixbefore", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-imultilib", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-isysroot", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("--sysroot"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nostdinc", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nostdinc++", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        // Library directories and libraries
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-L"),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-l"),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nostartfiles", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nodefaultlibs", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nostdlib", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nostdlib++", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static-libgcc", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static-libstdc++", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-shared", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-shared-libgcc", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-pie", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-rdynamic", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        // Preprocessor defines and undefines
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-D"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-U"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-include"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        // Output file
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-o"),
            ArgumentKind::Output,
        ),
        // Language and standard specification
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-std"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-x"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Dependency generation flags
        FlagRule::new(
            FlagPattern::Exactly("-M", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MM", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MD", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MMD", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MF", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MG", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MP", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MT", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MQ", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        // Linker flags
        FlagRule::new(
            FlagPattern::Prefix("-Wl,", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Xlinker", 1),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-T", 1),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-u", 1),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-z", 1),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        // Assembler flags
        FlagRule::new(
            FlagPattern::Prefix("-Wa,", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Xassembler", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Preprocessor flags
        FlagRule::new(
            FlagPattern::Prefix("-Wp,", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Xpreprocessor", 1),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        // Directory search flags
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-B"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Response files
        FlagRule::new(FlagPattern::Prefix("@", 0), ArgumentKind::Other(None)),
        // Plugin flags
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-fplugin"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Threading
        FlagRule::new(
            FlagPattern::Exactly("-pthread", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        // Profile and instrumentation
        FlagRule::new(
            FlagPattern::Exactly("-p", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-pg", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--coverage", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Preprocessor control flags
        FlagRule::new(
            FlagPattern::Exactly("-C", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-CC", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-P", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-traditional", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-traditional-cpp", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-trigraphs", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-undef", 0),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        // Dump flags - using prefix for comprehensive coverage
        FlagRule::new(
            FlagPattern::Prefix("-d", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Save temporary files
        FlagRule::new(
            FlagPattern::Prefix("-save-temps", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Specs files
        FlagRule::new(
            FlagPattern::ExactlyWithEqOrSep("-specs"),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        // Wrapper
        FlagRule::new(
            FlagPattern::Exactly("-wrapper", 1),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
    ];

    // Sort by flag length descending to ensure longer matches are tried first
    flags.sort_by(|a, b| b.pattern.flag().len().cmp(&a.pattern.flag().len()));

    flags
});

/// Parse command line arguments using the provided flag analyzer.
fn parse_arguments(flag_analyzer: &FlagAnalyzer, args: &[String]) -> Vec<Box<dyn Arguments>> {
    let mut result: Vec<Box<dyn Arguments>> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let remaining_args = &args[i..];

        // Handle the first argument (compiler name)
        if i == 0 {
            result.push(Box::new(OtherArguments::new(
                vec![args[0].clone()],
                ArgumentKind::Compiler,
            )));
            i += 1;
            continue;
        }

        let current_arg = &args[i];

        // Check if it's a source file (not a flag)
        if !current_arg.starts_with('-') {
            if looks_like_a_source_file(current_arg) {
                result.push(Box::new(SourceArgument::new(current_arg.clone())));
            } else {
                // Non-source, non-flag argument
                result.push(Box::new(OtherArguments::new(
                    vec![current_arg.clone()],
                    ArgumentKind::Other(None),
                )));
            }
            i += 1;
            continue;
        }

        // Try to match against flag definitions
        if let Some(match_result) = flag_analyzer.match_flag(remaining_args) {
            let consumed_count = match_result.consumed_args_count();
            let arg: Box<dyn Arguments> = match match_result.rule.kind {
                ArgumentKind::Compiler => Box::new(OtherArguments::new(
                    vec![match_result.consumed_args[0].clone()],
                    ArgumentKind::Compiler,
                )),
                ArgumentKind::Source => {
                    // This case should never occur since source files are handled by heuristic above
                    unreachable!("Source files should be detected by heuristic, not flag matching")
                }
                ArgumentKind::Output => match match_result.consumed_args_count() {
                    1 => Box::new(OutputArgument::new(
                        "-o".to_string(),
                        match_result.consumed_args[0][2..].to_string(),
                    )),
                    2 => Box::new(OutputArgument::new(
                        match_result.consumed_args[0].clone(),
                        match_result.consumed_args[1].clone(),
                    )),
                    _ => {
                        unreachable!("Output file should be specified either `-o file` or `-ofile`")
                    }
                },
                ArgumentKind::Other(compiler_pass) => Box::new(OtherArguments::new(
                    match_result.consumed_args,
                    ArgumentKind::Other(compiler_pass),
                )),
            };

            result.push(arg);
            i += consumed_count;
        } else {
            // Unknown flag - treat as simple flag
            result.push(Box::new(OtherArguments::new(
                vec![current_arg.clone()],
                ArgumentKind::Other(None),
            )));
            i += 1;
        }
    }

    result
}

/// Parse include directories from GCC-compatible environment variables.
///
/// https://gcc.gnu.org/onlinedocs/cpp/Environment-Variables.html
///
/// Returns a vector of Arguments representing the environment-based include directories.
/// This vector can be concatenated with the result of `parse_arguments` to create
/// a complete argument list.
fn parse_environment(
    environment: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Arguments>> {
    let mut args: Vec<Box<dyn Arguments>> = Vec::new();

    // Process the three GCC include environment variables that use -I
    for env_key in [
        KEY_GCC__C_INCLUDE_1,
        KEY_GCC__C_INCLUDE_2,
        KEY_GCC__C_INCLUDE_3,
    ] {
        if let Some(env_value) = environment.get(env_key) {
            // Use std::env::split_paths for platform-correct path splitting
            for path in std::env::split_paths(env_value) {
                if !path.as_os_str().is_empty() {
                    args.push(Box::new(OtherArguments::new(
                        vec!["-I".to_string(), path.to_string_lossy().to_string()],
                        ArgumentKind::Other(None),
                    )));
                }
            }
        }
    }

    // Process OBJC_INCLUDE_PATH which uses -isystem
    if let Some(env_value) = environment.get(KEY_GCC__OBJC_INCLUDE) {
        // Use std::env::split_paths for platform-correct path splitting
        for path in std::env::split_paths(env_value) {
            if !path.as_os_str().is_empty() {
                args.push(Box::new(OtherArguments::new(
                    vec!["-isystem".to_string(), path.to_string_lossy().to_string()],
                    ArgumentKind::Other(None),
                )));
            }
        }
    }

    args
}

/// Parse both command-line arguments and environment variables to generate complete argument list.
///
/// This is a convenience function that combines the results of `parse_arguments` and
/// `parse_environment`, providing a unified interface for both GCC and Clang interpreters.
///
/// # Arguments
///
/// * `flag_analyzer` - The flag analyzer to use for parsing command-line arguments
/// * `execution` - The execution context containing both arguments and environment variables
///
/// # Returns
///
/// A complete vector of Arguments containing both command-line and environment-based arguments.
pub fn parse_arguments_and_environment(
    flag_analyzer: &FlagAnalyzer,
    execution: &Execution,
) -> Vec<Box<dyn Arguments>> {
    let mut args = parse_arguments(flag_analyzer, &execution.arguments);
    let env_args = parse_environment(&execution.environment);
    args.extend(env_args);
    args
}

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
    fn test_simple_compilation() {
        let interpreter = GccInterpreter::new();
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-c", "main.c", "-o", "main.o"],
            "/project",
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);

            // Check compiler argument
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);

            // Check -c flag
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );

            // Check source file
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source);

            // Check output
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_combined_flags() {
        let interpreter = GccInterpreter::new();
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-I/usr/include", "-DDEBUG=1", "-o", "main", "main.c"],
            "/project",
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 5);

            // Check combined include flag
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-I/usr/include"]
            );

            // Check combined define flag
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-DDEBUG=1"]
            );

            // Check output
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
            assert_eq!(
                cmd.arguments[3].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-o", "main"]
            );

            // Check source
            assert_eq!(cmd.arguments[4].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_separate_flags() {
        let interpreter = GccInterpreter::new();
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-I", "/usr/include", "-D", "DEBUG=1", "main.c"],
            "/project",
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);

            // Check separate include flag
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-I", "/usr/include"]
            );

            // Check separate define flag
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[2].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-D", "DEBUG=1"]
            );

            // Check source
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Source);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_response_file() {
        let interpreter = GccInterpreter::new();
        let execution = create_execution("gcc", vec!["gcc", "@response.txt", "main.c"], "/project");

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 3);

            // Check response file
            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(None));
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["@response.txt"]
            );
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_warning_flags() {
        let interpreter = GccInterpreter::new();
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-Wall", "-Wextra", "-Wno-unused", "main.c"],
            "/project",
        );

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 5);

            // Check -Wall (exact match)
            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(None));
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-Wall"]
            );

            // Check -Wextra (exact match)
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Other(None));

            // Check -Wno-unused (prefix match with -W)
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Other(None));
            assert_eq!(
                cmd.arguments[3].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-Wno-unused"]
            );
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_std_flag_variations() {
        let interpreter = GccInterpreter::new();

        // Test separate form: -std c99
        let execution = create_execution("gcc", vec!["gcc", "-std", "c99", "main.c"], "/project");
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-std", "c99"]
            );
        }

        // Test equals form: -std=c99
        let execution = create_execution("gcc", vec!["gcc", "-std=c99", "main.c"], "/project");
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(
                cmd.arguments[1].as_arguments(&|p| Cow::Borrowed(p)),
                vec!["-std=c99"]
            );
        }
    }

    #[test]
    fn test_complex_compilation() {
        let interpreter = GccInterpreter::new();
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

        let result = interpreter.recognize(&execution).unwrap();

        if let Command::Compiler(cmd) = result {
            // Should parse into multiple argument groups
            assert!(cmd.arguments.len() >= 10);

            // Verify we have the right mix of argument types
            let mut source_count = 0;
            let mut output_count = 0;

            for arg in &cmd.arguments {
                match arg.kind() {
                    ArgumentKind::Source => source_count += 1,
                    ArgumentKind::Output => output_count += 1,
                    _ => {}
                }
            }

            assert_eq!(source_count, 2); // main.c, utils.c
            assert_eq!(output_count, 1); // -o program
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_argument_parsing_with_any_executable() {
        let interpreter = GccInterpreter::new();
        let execution = create_execution(
            "/usr/bin/any-compiler",
            vec!["any-compiler", "-c", "main.c", "-o", "main.o"],
            "/project",
        );

        let result = interpreter.recognize(&execution).unwrap();

        // Should always parse arguments regardless of executable name
        if let Command::Compiler(cmd) = result {
            assert_eq!(cmd.arguments.len(), 4);

            // Check that arguments are parsed correctly
            assert_eq!(cmd.arguments[0].kind(), ArgumentKind::Compiler);
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
            assert_eq!(cmd.arguments[2].kind(), ArgumentKind::Source);
            assert_eq!(cmd.arguments[3].kind(), ArgumentKind::Output);
        } else {
            panic!("Expected compiler command");
        }
    }

    #[test]
    fn test_comprehensive_flag_coverage() {
        let interpreter = GccInterpreter::new();

        // Test optimization flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-O2", "-Os", "-Ofast", "-Og", "main.c"],
            "/project",
        );
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // All -O* flags should be recognized
            assert!(cmd.arguments.len() >= 5);
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }
        }

        // Test debug flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec!["gcc", "-g", "-g3", "-gdwarf-4", "-ggdb", "main.c"],
            "/project",
        );
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // All -g* flags should be recognized
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }
        }

        // Test warning flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec![
                "gcc",
                "-Wall",
                "-Wextra",
                "-Wno-unused",
                "-Werror=format",
                "main.c",
            ],
            "/project",
        );
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // All -W* flags should be recognized
            for i in 1..5 {
                assert_eq!(cmd.arguments[i].kind(), ArgumentKind::Other(None));
            }
        }

        // Test feature flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec![
                "gcc",
                "-fPIC",
                "-fstack-protector",
                "-fno-omit-frame-pointer",
                "-flto",
                "main.c",
            ],
            "/project",
        );
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // All -f* flags should be recognized
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }
        }

        // Test machine flags with prefix matching
        let execution = create_execution(
            "gcc",
            vec![
                "gcc",
                "-m64",
                "-march=native",
                "-mtune=generic",
                "-msse4.2",
                "main.c",
            ],
            "/project",
        );
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // All -m* flags should be recognized
            for i in 1..5 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Compiling))
                );
            }
        }
    }

    #[test]
    fn test_linker_and_system_flags() {
        let interpreter = GccInterpreter::new();

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
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // Verify linker flags are recognized
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
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
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // Verify system paths are recognized with correct passes
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
            );
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
            );
        }
    }

    #[test]
    fn test_response_files_and_plugins() {
        let interpreter = GccInterpreter::new();

        // Test response files and plugins
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
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // All should be recognized as compilation flags
            assert_eq!(cmd.arguments[1].kind(), ArgumentKind::Other(None)); // @file
            assert_eq!(
                cmd.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            ); // plugin
            assert_eq!(
                cmd.arguments[3].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            ); // plugin-arg
            assert_eq!(
                cmd.arguments[4].kind(),
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            ); // save-temps
        }
    }

    #[test]
    fn test_environment_variables_cpath() {
        let interpreter = GccInterpreter::new();
        let cpath = create_path_string(&["/usr/include", "/opt/include"]);
        let mut env = HashMap::new();
        env.insert("CPATH", cpath.as_str());

        let execution = create_execution_with_env(
            "gcc",
            vec!["gcc", "-c", "main.c", "-o", "main.o"],
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
        let interpreter = GccInterpreter::new();
        let mut env = HashMap::new();
        env.insert("C_INCLUDE_PATH", "/usr/local/include");

        let execution = create_execution_with_env(
            "gcc",
            vec!["gcc", "-c", "main.c", "-o", "main.o"],
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
        let interpreter = GccInterpreter::new();
        let mut env = HashMap::new();
        env.insert("CPLUS_INCLUDE_PATH", "/usr/include/c++/11");

        let execution = create_execution_with_env(
            "g++",
            vec!["g++", "-c", "main.cpp", "-o", "main.o"],
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
        let interpreter = GccInterpreter::new();
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
    fn test_environment_variables_objc_include_path() {
        let interpreter = GccInterpreter::new();
        let objc_include_path =
            create_path_string(&["/System/Library/Frameworks", "/usr/local/objc"]);
        let mut env = HashMap::new();
        env.insert("OBJC_INCLUDE_PATH", objc_include_path.as_str());

        let execution = create_execution_with_env(
            "gcc",
            vec!["gcc", "-c", "main.m", "-o", "main.o"],
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
        let interpreter = GccInterpreter::new();
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

    #[test]
    fn test_environment_variables_empty_paths() {
        let interpreter = GccInterpreter::new();
        let c_include_path = create_path_string(&["", "", "", ""]);
        let mut env = HashMap::new();
        env.insert("CPATH", "");
        env.insert("C_INCLUDE_PATH", c_include_path.as_str()); // Empty paths should be filtered out

        let execution = create_execution_with_env(
            "gcc",
            vec!["gcc", "-c", "main.c", "-o", "main.o"],
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
    fn test_preprocessor_comprehensive_flags() {
        let interpreter = GccInterpreter::new();

        // Test comprehensive preprocessor flags
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
        let result = interpreter.recognize(&execution).unwrap();
        if let Command::Compiler(cmd) = result {
            // Verify preprocessor flags are correctly categorized
            assert_eq!(
                cmd.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            // Most preprocessor control flags should be preprocessing
            for i in 2..13 {
                assert_eq!(
                    cmd.arguments[i].kind(),
                    ArgumentKind::Other(Some(CompilerPass::Preprocessing))
                );
            }
        }
    }
}

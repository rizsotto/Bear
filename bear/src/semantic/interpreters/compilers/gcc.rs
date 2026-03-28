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

use super::super::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use super::arguments::{OtherArguments, OutputArgument, SourceArgument};
use crate::environment::{
    KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3, KEY_GCC__OBJC_INCLUDE,
};
use crate::semantic::{
    ArgumentKind, Arguments, Command, CompilerCommand, CompilerPass, Execution, Interpreter, PassEffect,
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
        Self { matcher: FlagAnalyzer::new(&GCC_FLAGS) }
    }

    /// Check if the execution represents a GCC internal executable that should be ignored.
    fn is_gcc_internal_executable(execution: &Execution) -> bool {
        if let Some(filename) = execution.executable.file_name()
            && let Some(filename_str) = filename.to_str()
        {
            return GCC_INTERNAL_EXECUTABLES.contains(&filename_str);
        }
        false
    }
}

/// GCC internal executables that should be ignored
/// These are implementation details of GCC's compilation process
const GCC_INTERNAL_EXECUTABLES: [&str; 7] = [
    "cc1",        // C compiler proper
    "cc1plus",    // C++ compiler proper
    "cc1obj",     // Objective-C compiler proper
    "cc1objplus", // Objective-C++ compiler proper
    "f951",       // Fortran compiler proper
    "collect2",   // Linker wrapper
    "lto1",       // Link-time optimization pass
];

impl Interpreter for GccInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // Check if this is a GCC internal executable that should be ignored
        if Self::is_gcc_internal_executable(execution) {
            return Some(Command::Ignored("GCC internal executable"));
        }

        // Parse both command-line arguments and environment variables
        let annotated_args = parse_arguments_and_environment(&self.matcher, execution);

        Some(Command::Compiler(CompilerCommand::new(
            execution.working_dir.clone(),
            execution.executable.clone(),
            annotated_args,
        )))
    }
}

// GCC flag definitions. Generated at build time from flags/gcc.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_gcc.rs"));

/// Parse command line arguments using the provided flag analyzer.
fn parse_arguments(flag_analyzer: &FlagAnalyzer, args: &[String]) -> Vec<Box<dyn Arguments>> {
    let mut result: Vec<Box<dyn Arguments>> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let remaining_args = &args[i..];

        // Handle the first argument (compiler name)
        if i == 0 {
            result.push(Box::new(OtherArguments::new(vec![args[0].clone()], ArgumentKind::Compiler)));
            i += 1;
            continue;
        }

        let current_arg = &args[i];

        // Try to match against flag definitions first (handles both -flags and @response files)
        if let Some(match_result) = flag_analyzer.match_flag(remaining_args) {
            let consumed_count = match_result.consumed_args_count();
            let arg: Box<dyn Arguments> = match match_result.rule.kind {
                ArgumentKind::Compiler => Box::new(OtherArguments::new(
                    vec![match_result.consumed_args[0].clone()],
                    ArgumentKind::Compiler,
                )),
                ArgumentKind::Source { .. } => {
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
            if current_arg.starts_with('-') {
                // Unknown flag - treat as simple flag
                result.push(Box::new(OtherArguments::new(
                    vec![current_arg.clone()],
                    ArgumentKind::Other(PassEffect::None),
                )));
            } else {
                // non-flag argument (e.g., source files, object files, libraries)
                result.push(Box::new(SourceArgument::new(current_arg.clone())));
            }
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
fn parse_environment(environment: &std::collections::HashMap<String, String>) -> Vec<Box<dyn Arguments>> {
    let mut args: Vec<Box<dyn Arguments>> = Vec::new();

    // Process the three GCC include environment variables that use -I
    for env_key in [KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3] {
        if let Some(env_value) = environment.get(env_key) {
            // Use std::env::split_paths for platform-correct path splitting
            for path in std::env::split_paths(env_value) {
                if !path.as_os_str().is_empty() {
                    args.push(Box::new(OtherArguments::new(
                        vec!["-I".to_string(), path.to_string_lossy().to_string()],
                        ArgumentKind::Other(PassEffect::None),
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
                    ArgumentKind::Other(PassEffect::None),
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

/// Validate semantic invariants that all flag tables must satisfy.
/// Used by tests in this module and other compiler modules.
#[cfg(test)]
pub fn assert_flag_table_invariants(flags: &[FlagRule]) {
    assert!(!flags.is_empty(), "Flag table must not be empty");

    // Sorted by flag length descending
    for window in flags.windows(2) {
        assert!(
            window[0].pattern.flag().len() >= window[1].pattern.flag().len(),
            "Flags not sorted by length: {:?} (len {}) before {:?} (len {})",
            window[0].pattern.flag(),
            window[0].pattern.flag().len(),
            window[1].pattern.flag(),
            window[1].pattern.flag().len(),
        );
    }

    for rule in flags {
        // No flag rule uses ArgumentKind::Source (source files detected by heuristic)
        assert!(
            !matches!(rule.kind, ArgumentKind::Source { .. }),
            "Flag rule {:?} must not use ArgumentKind::Source",
            rule.pattern.flag()
        );

        // All flags start with '-', '--', or '@'
        let flag = rule.pattern.flag();
        assert!(flag.starts_with('-') || flag.starts_with('@'), "Flag {:?} must start with '-' or '@'", flag);

        // Output rules can only produce 1 or 2 consumed args
        if matches!(rule.kind, ArgumentKind::Output) {
            match rule.pattern {
                FlagPattern::Exactly(_, n) => {
                    assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                }
                FlagPattern::ExactlyWithEq(_)
                | FlagPattern::ExactlyWithEqOrSep(_)
                | FlagPattern::ExactlyWithGluedOrSep(_) => {} // always 1 or 2
                FlagPattern::Prefix(_, n) => {
                    assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_table_invariants() {
        assert_flag_table_invariants(&GCC_FLAGS);
    }
}

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
use super::gcc::parse_arguments_and_environment;
use crate::semantic::{
    ArgumentKind, Command, CompilerCommand, CompilerPass, Execution, Interpreter, PassEffect,
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
        Self { matcher: FlagAnalyzer::new(&CLANG_FLAGS) }
    }

    /// Checks if the execution is an internal clang -cc1 frontend invocation.
    /// These are internal compiler calls that happen after the user-facing command.
    fn is_cc1_invocation(execution: &Execution) -> bool {
        execution.arguments.iter().any(|arg| arg == "-cc1")
    }
}

impl Interpreter for ClangInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        // Skip internal clang -cc1 invocations (clang's internal frontend)
        // These are internal compiler calls that happen after the user-facing command
        if Self::is_cc1_invocation(execution) {
            return Some(Command::Ignored("clang internal invocation"));
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

// Clang flag definitions. Generated at build time from flags/clang.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_clang.rs"));

/// Flang (Fortran) compiler interpreter.
///
/// This interpreter recognizes Flang Fortran compiler commands and their associated flags.
/// It extends the base GCC flag set with Fortran-specific flags and patterns.
pub struct FlangInterpreter {
    /// Flag analyzer that recognizes and categorizes Flang command-line flags
    /// (includes GCC-compatible flags plus Flang-specific extensions)
    matcher: FlagAnalyzer,
}

impl Default for FlangInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl FlangInterpreter {
    /// Creates a new Flang interpreter with comprehensive Fortran flag definitions.
    ///
    /// This combines Flang-specific flags with the base GCC flag set to provide
    /// complete Fortran compilation command recognition.
    pub fn new() -> Self {
        Self { matcher: FlagAnalyzer::new(&FLANG_FLAGS) }
    }
}

impl Interpreter for FlangInterpreter {
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

// Flang flag definitions. Generated at build time from flags/flang.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_flang.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::interpreters::compilers::gcc::GCC_FLAGS;

    #[test]
    fn test_gcc_flag_inheritance() {
        let gcc_flags = &GCC_FLAGS;
        let clang_flags = &CLANG_FLAGS;

        assert!(
            clang_flags.len() > gcc_flags.len(),
            "Clang should have more flags than GCC, got gcc: {}, clang: {}",
            gcc_flags.len(),
            clang_flags.len()
        );

        let gcc_flag_strings: std::collections::HashSet<&str> =
            gcc_flags.iter().map(|f| f.pattern.flag()).collect();
        let clang_flag_strings: std::collections::HashSet<&str> =
            clang_flags.iter().map(|f| f.pattern.flag()).collect();

        let missing_flags: Vec<&str> = gcc_flag_strings.difference(&clang_flag_strings).cloned().collect();
        assert!(missing_flags.is_empty(), "These GCC flags are missing from Clang: {:?}", missing_flags);
    }

    #[test]
    fn test_flag_table_invariants() {
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&CLANG_FLAGS);
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&FLANG_FLAGS);
    }
}

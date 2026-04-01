// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic flag-based compiler interpreter.
//!
//! This module provides a single interpreter type that handles all flag-table-driven
//! compilers (GCC, Clang, Flang, CUDA, Intel Fortran, Cray Fortran). Each compiler
//! is parameterized by its generated flag table and optional ignore filters, eliminating
//! the need for per-compiler structs and trait implementations.

use super::super::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use super::arguments::{OtherArguments, OutputArgument, SourceArgument};
use crate::environment::{
    KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3, KEY_GCC__OBJC_INCLUDE,
};
use crate::semantic::{
    ArgumentKind, Arguments, Command, CompilerCommand, CompilerPass, Execution, Interpreter, PassEffect,
};

/// A generic compiler interpreter parameterized by a flag table and ignore filters.
///
/// This replaces the individual per-compiler interpreter structs (GccInterpreter,
/// ClangInterpreter, etc.) with a single type driven by build-time-generated data.
struct FlagBasedInterpreter {
    analyzer: FlagAnalyzer,
    ignore_executables: &'static [&'static str],
    ignore_flags: &'static [&'static str],
}

impl FlagBasedInterpreter {
    /// Creates a new flag-based interpreter with the given flag table and ignore filters.
    fn new(
        flags: &'static [FlagRule],
        ignore_executables: &'static [&'static str],
        ignore_flags: &'static [&'static str],
    ) -> Self {
        Self { analyzer: FlagAnalyzer::new(flags), ignore_executables, ignore_flags }
    }

    fn should_ignore(&self, execution: &Execution) -> Option<&'static str> {
        // Check executable name against ignore list
        if !self.ignore_executables.is_empty()
            && let Some(filename) = execution.executable.file_name()
            && let Some(filename_str) = filename.to_str()
            && self.ignore_executables.contains(&filename_str)
        {
            return Some("internal executable");
        }

        // Check arguments against ignore flags
        if !self.ignore_flags.is_empty()
            && self.ignore_flags.iter().any(|flag| execution.arguments.iter().any(|arg| arg == flag))
        {
            return Some("internal invocation");
        }

        None
    }
}

impl Interpreter for FlagBasedInterpreter {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        if let Some(reason) = self.should_ignore(execution) {
            return Some(Command::Ignored(reason));
        }

        let annotated_args = parse_arguments_and_environment(&self.analyzer, execution);

        Some(Command::Compiler(CompilerCommand::new(
            execution.working_dir.clone(),
            execution.executable.clone(),
            annotated_args,
        )))
    }
}

/// Parse both command-line arguments and environment variables to generate complete argument list.
fn parse_arguments_and_environment(
    flag_analyzer: &FlagAnalyzer,
    execution: &Execution,
) -> Vec<Box<dyn Arguments>> {
    let mut args = parse_arguments(flag_analyzer, &execution.arguments);
    let env_args = parse_environment(&execution.environment);
    args.extend(env_args);
    args
}

/// Parse command line arguments using the provided flag analyzer.
fn parse_arguments(flag_analyzer: &FlagAnalyzer, args: &[String]) -> Vec<Box<dyn Arguments>> {
    let mut result: Vec<Box<dyn Arguments>> = Vec::with_capacity(args.len());
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
fn parse_environment(environment: &std::collections::HashMap<String, String>) -> Vec<Box<dyn Arguments>> {
    let mut args: Vec<Box<dyn Arguments>> = Vec::with_capacity(4);

    // Process the three GCC include environment variables that use -I
    for env_key in [KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3] {
        if let Some(env_value) = environment.get(env_key) {
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

// Flag tables and ignore arrays. Generated at build time from flags/*.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_gcc.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_clang.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_flang.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_cuda.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_intel_fortran.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_cray_fortran.rs"));

/// Factory functions returning opaque interpreters so callers never see concrete types.
pub(super) fn gcc() -> impl Interpreter {
    FlagBasedInterpreter::new(&GCC_FLAGS, &GCC_IGNORE_EXECUTABLES, &GCC_IGNORE_FLAGS)
}

pub(super) fn clang() -> impl Interpreter {
    FlagBasedInterpreter::new(&CLANG_FLAGS, &CLANG_IGNORE_EXECUTABLES, &CLANG_IGNORE_FLAGS)
}

pub(super) fn flang() -> impl Interpreter {
    FlagBasedInterpreter::new(&FLANG_FLAGS, &FLANG_IGNORE_EXECUTABLES, &FLANG_IGNORE_FLAGS)
}

pub(super) fn cuda() -> impl Interpreter {
    FlagBasedInterpreter::new(&CUDA_FLAGS, &CUDA_IGNORE_EXECUTABLES, &CUDA_IGNORE_FLAGS)
}

pub(super) fn intel_fortran() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &INTEL_FORTRAN_FLAGS,
        &INTEL_FORTRAN_IGNORE_EXECUTABLES,
        &INTEL_FORTRAN_IGNORE_FLAGS,
    )
}

pub(super) fn cray_fortran() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &CRAY_FORTRAN_FLAGS,
        &CRAY_FORTRAN_IGNORE_EXECUTABLES,
        &CRAY_FORTRAN_IGNORE_FLAGS,
    )
}

#[cfg(test)]
mod flag_table_invariants {
    use super::*;

    fn assert_invariants(flags: &[FlagRule]) {
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
            assert!(
                !matches!(rule.kind, ArgumentKind::Source { .. }),
                "Flag rule {:?} must not use ArgumentKind::Source",
                rule.pattern.flag()
            );

            let flag = rule.pattern.flag();
            assert!(
                flag.starts_with('-') || flag.starts_with('@'),
                "Flag {:?} must start with '-' or '@'",
                flag
            );

            if matches!(rule.kind, ArgumentKind::Output) {
                match rule.pattern {
                    FlagPattern::Exactly(_, n) => {
                        assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                    }
                    FlagPattern::ExactlyWithEq(_)
                    | FlagPattern::ExactlyWithEqOrSep(_)
                    | FlagPattern::ExactlyWithGluedOrSep(_) => {}
                    FlagPattern::Prefix(_, n) => {
                        assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                    }
                }
            }
        }
    }

    #[test]
    fn gcc() {
        assert_invariants(&GCC_FLAGS);
    }

    #[test]
    fn clang() {
        assert_invariants(&CLANG_FLAGS);
    }

    #[test]
    fn flang() {
        assert_invariants(&FLANG_FLAGS);
    }

    #[test]
    fn cuda() {
        assert_invariants(&CUDA_FLAGS);
    }

    #[test]
    fn intel_fortran() {
        assert_invariants(&INTEL_FORTRAN_FLAGS);
    }

    #[test]
    fn cray_fortran() {
        assert_invariants(&CRAY_FORTRAN_FLAGS);
    }

    #[test]
    fn clang_inherits_all_gcc_flags() {
        let gcc_flag_strings: std::collections::HashSet<&str> =
            GCC_FLAGS.iter().map(|f| f.pattern.flag()).collect();
        let clang_flag_strings: std::collections::HashSet<&str> =
            CLANG_FLAGS.iter().map(|f| f.pattern.flag()).collect();

        assert!(
            CLANG_FLAGS.len() > GCC_FLAGS.len(),
            "Clang should have more flags than GCC, got gcc: {}, clang: {}",
            GCC_FLAGS.len(),
            CLANG_FLAGS.len()
        );

        let missing_flags: Vec<&str> = gcc_flag_strings.difference(&clang_flag_strings).cloned().collect();
        assert!(missing_flags.is_empty(), "These GCC flags are missing from Clang: {:?}", missing_flags);
    }
}

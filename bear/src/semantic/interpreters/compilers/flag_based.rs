// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic flag-based compiler interpreter.
//!
//! This module provides a single interpreter type that handles all flag-table-driven
//! compilers (GCC, Clang, Flang, CUDA, Intel Fortran, Cray Fortran). Each compiler
//! is parameterized by its generated flag table and optional ignore filters, eliminating
//! the need for per-compiler structs and trait implementations.

use super::super::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use crate::environment::{
    KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3, KEY_GCC__OBJC_INCLUDE,
};
use crate::semantic::{
    Argument, ArgumentKind, Command, CompilerPass, Execution, Interpreter, PassEffect, RecognizeResult,
};

/// A generic compiler interpreter parameterized by a flag table and ignore filters.
///
/// This replaces the individual per-compiler interpreter structs (GccInterpreter,
/// ClangInterpreter, etc.) with a single type driven by build-time-generated data.
struct FlagBasedInterpreter {
    analyzer: FlagAnalyzer,
    ignore_executables: &'static [&'static str],
    ignore_flags: &'static [&'static str],
    /// When true, arguments starting with '/' are treated as flags (MSVC-style).
    /// When false (default), only '-' prefixed arguments are treated as flags.
    slash_prefix: bool,
}

impl FlagBasedInterpreter {
    /// Creates a new flag-based interpreter with the given flag table and ignore filters.
    fn new(
        flags: &'static [FlagRule],
        ignore_executables: &'static [&'static str],
        ignore_flags: &'static [&'static str],
        slash_prefix: bool,
    ) -> Self {
        Self { analyzer: FlagAnalyzer::new(flags), ignore_executables, ignore_flags, slash_prefix }
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
    fn recognize(&self, execution: Execution) -> RecognizeResult {
        if let Some(reason) = self.should_ignore(&execution) {
            return RecognizeResult::Ignored(reason);
        }

        let Execution { executable, mut arguments, working_dir, environment } = execution;
        let annotated_args = parse_arguments_owned(&self.analyzer, &mut arguments, self.slash_prefix);
        let env_args = parse_environment(&environment);

        let mut all_args = annotated_args;
        all_args.extend(env_args);

        RecognizeResult::Recognized(Command::new(working_dir, executable, all_args))
    }
}

/// Parse command line arguments, moving strings out of the owned Vec.
///
/// Uses `std::mem::take` to move strings into Argument variants without cloning.
/// The source Vec elements become empty strings after being taken.
fn parse_arguments_owned(
    flag_analyzer: &FlagAnalyzer,
    args: &mut [String],
    slash_prefix: bool,
) -> Vec<Argument> {
    let mut result: Vec<Argument> = Vec::with_capacity(args.len());
    let mut i = 0;

    while i < args.len() {
        // Handle the first argument (compiler name)
        if i == 0 {
            result.push(Argument::Other {
                arguments: vec![std::mem::take(&mut args[0])],
                kind: ArgumentKind::Compiler,
            });
            i += 1;
            continue;
        }

        // match_flag needs a view of the remaining args; taken slots are behind us
        let remaining_args = &args[i..];

        if let Some(match_result) = flag_analyzer.match_flag(remaining_args) {
            // Handle pass-through first (early exit)
            if matches!(match_result.rule.kind, ArgumentKind::Other(PassEffect::PassThrough)) {
                result.push(Argument::Other {
                    arguments: vec![std::mem::take(&mut args[i])],
                    kind: ArgumentKind::Other(PassEffect::PassThrough),
                });
                i += 1;
                while i < args.len() {
                    result.push(Argument::Other {
                        arguments: vec![std::mem::take(&mut args[i])],
                        kind: ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
                    });
                    i += 1;
                }
                break;
            }

            let consumed_count = match_result.consumed_count;
            let arg = match match_result.rule.kind {
                ArgumentKind::Compiler => Argument::Other {
                    arguments: vec![std::mem::take(&mut args[i])],
                    kind: ArgumentKind::Compiler,
                },
                ArgumentKind::Source { .. } => {
                    unreachable!("Source files should be detected by heuristic, not flag matching")
                }
                ArgumentKind::Output => match consumed_count {
                    1 => {
                        let val = std::mem::take(&mut args[i]);
                        let flag_str = match_result.rule.pattern.flag();
                        let after_flag = &val[flag_str.len()..];
                        // Skip separator character (= or :) if present
                        let path = if after_flag.starts_with('=') || after_flag.starts_with(':') {
                            after_flag[1..].to_string()
                        } else {
                            after_flag.to_string()
                        };
                        Argument::Output { flag: flag_str.to_string(), path }
                    }
                    2 => Argument::Output {
                        flag: std::mem::take(&mut args[i]),
                        path: std::mem::take(&mut args[i + 1]),
                    },
                    _ => {
                        unreachable!("Output file should be specified with glued or separate value")
                    }
                },
                ArgumentKind::Other(compiler_pass) => {
                    let moved: Vec<String> =
                        (i..i + consumed_count).map(|j| std::mem::take(&mut args[j])).collect();
                    Argument::Other { arguments: moved, kind: ArgumentKind::Other(compiler_pass) }
                }
            };

            result.push(arg);
            i += consumed_count;
        } else if args[i].starts_with('-') || (slash_prefix && args[i].starts_with('/')) {
            result.push(Argument::Other {
                arguments: vec![std::mem::take(&mut args[i])],
                kind: ArgumentKind::Other(PassEffect::None),
            });
            i += 1;
        } else {
            result.push(Argument::new_source(std::mem::take(&mut args[i])));
            i += 1;
        }
    }

    result
}

/// Parse include directories from GCC-compatible environment variables.
///
/// https://gcc.gnu.org/onlinedocs/cpp/Environment-Variables.html
fn parse_environment(environment: &std::collections::HashMap<String, String>) -> Vec<Argument> {
    let mut args: Vec<Argument> = Vec::with_capacity(4);

    for env_key in [KEY_GCC__C_INCLUDE_1, KEY_GCC__C_INCLUDE_2, KEY_GCC__C_INCLUDE_3] {
        if let Some(env_value) = environment.get(env_key) {
            for path in std::env::split_paths(env_value) {
                if !path.as_os_str().is_empty() {
                    args.push(Argument::Other {
                        arguments: vec!["-I".to_string(), path.to_string_lossy().to_string()],
                        kind: ArgumentKind::Other(PassEffect::None),
                    });
                }
            }
        }
    }

    if let Some(env_value) = environment.get(KEY_GCC__OBJC_INCLUDE) {
        for path in std::env::split_paths(env_value) {
            if !path.as_os_str().is_empty() {
                args.push(Argument::Other {
                    arguments: vec!["-isystem".to_string(), path.to_string_lossy().to_string()],
                    kind: ArgumentKind::Other(PassEffect::None),
                });
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
include!(concat!(env!("OUT_DIR"), "/flags_msvc.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_clang_cl.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_intel_cc.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_nvidia_hpc.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_armclang.rs"));
include!(concat!(env!("OUT_DIR"), "/flags_ibm_xl.rs"));

/// Factory functions returning opaque interpreters so callers never see concrete types.
pub(super) fn gcc() -> impl Interpreter {
    FlagBasedInterpreter::new(&GCC_FLAGS, &GCC_IGNORE_EXECUTABLES, &GCC_IGNORE_FLAGS, GCC_SLASH_PREFIX)
}

pub(super) fn clang() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &CLANG_FLAGS,
        &CLANG_IGNORE_EXECUTABLES,
        &CLANG_IGNORE_FLAGS,
        CLANG_SLASH_PREFIX,
    )
}

pub(super) fn flang() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &FLANG_FLAGS,
        &FLANG_IGNORE_EXECUTABLES,
        &FLANG_IGNORE_FLAGS,
        FLANG_SLASH_PREFIX,
    )
}

pub(super) fn cuda() -> impl Interpreter {
    FlagBasedInterpreter::new(&CUDA_FLAGS, &CUDA_IGNORE_EXECUTABLES, &CUDA_IGNORE_FLAGS, CUDA_SLASH_PREFIX)
}

pub(super) fn intel_fortran() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &INTEL_FORTRAN_FLAGS,
        &INTEL_FORTRAN_IGNORE_EXECUTABLES,
        &INTEL_FORTRAN_IGNORE_FLAGS,
        INTEL_FORTRAN_SLASH_PREFIX,
    )
}

pub(super) fn cray_fortran() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &CRAY_FORTRAN_FLAGS,
        &CRAY_FORTRAN_IGNORE_EXECUTABLES,
        &CRAY_FORTRAN_IGNORE_FLAGS,
        CRAY_FORTRAN_SLASH_PREFIX,
    )
}

pub(super) fn msvc() -> impl Interpreter {
    FlagBasedInterpreter::new(&MSVC_FLAGS, &MSVC_IGNORE_EXECUTABLES, &MSVC_IGNORE_FLAGS, MSVC_SLASH_PREFIX)
}

pub(super) fn clang_cl() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &CLANG_CL_FLAGS,
        &CLANG_CL_IGNORE_EXECUTABLES,
        &CLANG_CL_IGNORE_FLAGS,
        CLANG_CL_SLASH_PREFIX,
    )
}

pub(super) fn intel_cc() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &INTEL_CC_FLAGS,
        &INTEL_CC_IGNORE_EXECUTABLES,
        &INTEL_CC_IGNORE_FLAGS,
        INTEL_CC_SLASH_PREFIX,
    )
}

pub(super) fn nvidia_hpc() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &NVIDIA_HPC_FLAGS,
        &NVIDIA_HPC_IGNORE_EXECUTABLES,
        &NVIDIA_HPC_IGNORE_FLAGS,
        NVIDIA_HPC_SLASH_PREFIX,
    )
}

pub(super) fn armclang() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &ARMCLANG_FLAGS,
        &ARMCLANG_IGNORE_EXECUTABLES,
        &ARMCLANG_IGNORE_FLAGS,
        ARMCLANG_SLASH_PREFIX,
    )
}

pub(super) fn ibm_xl() -> impl Interpreter {
    FlagBasedInterpreter::new(
        &IBM_XL_FLAGS,
        &IBM_XL_IGNORE_EXECUTABLES,
        &IBM_XL_IGNORE_FLAGS,
        IBM_XL_SLASH_PREFIX,
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
                flag.starts_with('-') || flag.starts_with('@') || flag.starts_with('/'),
                "Flag {:?} must start with '-', '@', or '/'",
                flag
            );

            if matches!(rule.kind, ArgumentKind::Output) {
                match rule.pattern {
                    FlagPattern::Exactly(_, n) => {
                        assert!(n <= 1, "Output rule {:?} must take 0 or 1 extra args", flag)
                    }
                    FlagPattern::ExactlyWithEq(_)
                    | FlagPattern::ExactlyWithEqOrSep(_)
                    | FlagPattern::ExactlyWithColon(_)
                    | FlagPattern::ExactlyWithColonOrSep(_)
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
    fn msvc() {
        assert_invariants(&MSVC_FLAGS);
    }

    #[test]
    fn clang_cl() {
        assert_invariants(&CLANG_CL_FLAGS);
    }

    #[test]
    fn intel_cc() {
        assert_invariants(&INTEL_CC_FLAGS);
    }

    #[test]
    fn nvidia_hpc() {
        assert_invariants(&NVIDIA_HPC_FLAGS);
    }

    #[test]
    fn armclang() {
        assert_invariants(&ARMCLANG_FLAGS);
    }

    #[test]
    fn ibm_xl() {
        assert_invariants(&IBM_XL_FLAGS);
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

#[cfg(test)]
mod pass_through_tests {
    use super::*;
    use crate::semantic::interpreters::matchers::{FlagAnalyzer, FlagPattern, FlagRule};

    #[test]
    fn test_pass_through_flag_stops_parsing() {
        static PASS_THROUGH_FLAGS: std::sync::LazyLock<Vec<FlagRule>> = std::sync::LazyLock::new(|| {
            let mut flags = vec![
                FlagRule::new(
                    FlagPattern::Exactly("-c", 0),
                    ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
                ),
                FlagRule::new(FlagPattern::Exactly("/link", 0), ArgumentKind::Other(PassEffect::PassThrough)),
                FlagRule::new(FlagPattern::Exactly("-o", 1), ArgumentKind::Output),
            ];
            flags.sort_by(|a, b| b.pattern.flag().len().cmp(&a.pattern.flag().len()));
            flags
        });

        let analyzer = FlagAnalyzer::new(&PASS_THROUGH_FLAGS);
        let mut args = vec![
            "cl".to_string(),
            "-c".to_string(),
            "foo.c".to_string(),
            "/link".to_string(),
            "/SUBSYSTEM:CONSOLE".to_string(),
            "/OUT:foo.exe".to_string(),
        ];

        let result = parse_arguments_owned(&analyzer, &mut args, false);

        // cl (compiler)
        assert!(matches!(result[0], Argument::Other { ref kind, .. } if *kind == ArgumentKind::Compiler));
        // -c (stops at compiling)
        assert!(
            matches!(result[1], Argument::Other { ref kind, .. } if *kind == ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)))
        );
        // foo.c (source)
        assert!(matches!(result[2], Argument::Source { .. }));
        // /link (pass-through marker)
        assert!(
            matches!(result[3], Argument::Other { ref kind, .. } if *kind == ArgumentKind::Other(PassEffect::PassThrough))
        );
        // /SUBSYSTEM:CONSOLE (linker arg)
        assert!(
            matches!(result[4], Argument::Other { ref kind, .. } if *kind == ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)))
        );
        // /OUT:foo.exe (linker arg)
        assert!(
            matches!(result[5], Argument::Other { ref kind, .. } if *kind == ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)))
        );
    }
}

#[cfg(test)]
mod slash_prefix_tests {
    use super::*;
    use crate::semantic::interpreters::matchers::{FlagAnalyzer, FlagRule};

    #[test]
    fn slash_prefixed_args_treated_as_source_without_slash_support() {
        let flags: &[FlagRule] = &[];
        let analyzer = FlagAnalyzer::new(flags);
        let mut args = vec!["cl".to_string(), "/c".to_string(), "foo.c".to_string()];
        let result = parse_arguments_owned(&analyzer, &mut args, false);
        // /c should be a source file since slash_prefix is false
        assert!(matches!(result[1], Argument::Source { .. }));
    }

    #[test]
    fn slash_prefixed_args_treated_as_flags_with_slash_support() {
        let flags: &[FlagRule] = &[];
        let analyzer = FlagAnalyzer::new(flags);
        let mut args = vec!["cl".to_string(), "/c".to_string(), "foo.c".to_string()];
        let result = parse_arguments_owned(&analyzer, &mut args, true);
        // /c should be an unrecognized flag (Other with None) since slash_prefix is true
        assert!(matches!(
            result[1],
            Argument::Other { ref kind, .. } if *kind == ArgumentKind::Other(PassEffect::None)
        ));
    }

    #[test]
    fn output_extraction_works_with_glued_eq() {
        use crate::semantic::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> =
            LazyLock::new(|| vec![FlagRule::new(FlagPattern::ExactlyWithEq("-o"), ArgumentKind::Output)]);

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let mut args = vec!["gcc".to_string(), "-o=foo.o".to_string()];
        let result = parse_arguments_owned(&analyzer, &mut args, false);
        assert!(
            matches!(result[1], Argument::Output { ref flag, ref path } if flag == "-o" && path == "foo.o")
        );
    }

    #[test]
    fn output_extraction_works_with_glued_colon() {
        use crate::semantic::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> =
            LazyLock::new(|| vec![FlagRule::new(FlagPattern::ExactlyWithColon("/Fo"), ArgumentKind::Output)]);

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let mut args = vec!["cl".to_string(), "/Fo:foo.obj".to_string()];
        let result = parse_arguments_owned(&analyzer, &mut args, true);
        assert!(
            matches!(result[1], Argument::Output { ref flag, ref path } if flag == "/Fo" && path == "foo.obj")
        );
    }

    #[test]
    fn output_extraction_works_with_glued_value() {
        use crate::semantic::interpreters::matchers::FlagPattern;
        use std::sync::LazyLock;

        static OUTPUT_FLAGS: LazyLock<Vec<FlagRule>> = LazyLock::new(|| {
            vec![FlagRule::new(FlagPattern::ExactlyWithGluedOrSep("-o"), ArgumentKind::Output)]
        });

        let analyzer = FlagAnalyzer::new(&OUTPUT_FLAGS);
        let mut args = vec!["gcc".to_string(), "-ofoo.o".to_string()];
        let result = parse_arguments_owned(&analyzer, &mut args, false);
        assert!(
            matches!(result[1], Argument::Output { ref flag, ref path } if flag == "-o" && path == "foo.o")
        );
    }
}

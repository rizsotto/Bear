// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler wrapper handling for ccache, distcc, sccache, and icecc.
//!
//! Wrappers sit between the build system and the real compiler:
//! `ccache gcc -c main.c`. The job here is small: detect the wrapper by
//! basename, locate the real compiler in argv (skipping wrapper-specific
//! flags like `distcc -j 4`), and produce a fresh [`Execution`] that
//! names the real compiler. The caller (`CompilerInterpreter::recognize`)
//! then dispatches that execution as if the wrapper had never been there.
//!
//! This module is the authority on what counts as a wrapper:
//! [`WRAPPER_NAMES`] is shared with `compiler_recognition` for the
//! recognizer's regex pattern and probe guard.

use super::compiler_recognition::CompilerRecognizer;
use crate::config::CompilerType;
use intercept::Execution;

use std::path::{Path, PathBuf};

/// Wrapper executable basenames. Single source of truth; consumed by
/// [`CompilerRecognizer`] to build the regex that classifies wrappers and
/// to skip them during the `--version` probe.
///
/// icecream's `icerun` is deliberately absent: it launches arbitrary
/// commands on the cluster, not compiler invocations.
pub(super) const WRAPPER_NAMES: &[&str] = &["ccache", "distcc", "sccache", "icecc"];

/// Try to strip a wrapper from `execution`, returning the inner compiler
/// invocation along with its recognized [`CompilerType`].
///
/// Returns `Ok((inner, ty))` when `execution` is a wrapper invocation we
/// recognize and the inner argv names a real (non-wrapper) compiler;
/// `ty` is the inner compiler's type so the caller can dispatch without
/// re-running the recognizer.
///
/// Returns `Err(execution)` -- handing the original execution back -- in
/// every other case: not a wrapper, missing inner argument, inner not a
/// compiler, or wrapper-of-wrapper (e.g. `ccache distcc gcc`). The Err
/// arm lets the caller surface `RecognizeResult::NotRecognized` without
/// re-cloning the execution.
pub(super) fn unwrap(
    execution: Execution,
    recognizer: &CompilerRecognizer,
) -> Result<(Execution, CompilerType), Execution> {
    let Some(wrapper_name) = detect_wrapper_name(&execution.executable) else {
        return Err(execution);
    };

    let Some((real_compiler, filtered_args)) = extract_real_compiler(wrapper_name, &execution.arguments)
    else {
        return Err(execution);
    };

    // The inner argv must name a real compiler; reject wrapper-of-wrapper
    // (which would otherwise loop) and unknown executables.
    let inner_type = match recognizer.recognize(&real_compiler) {
        Some(CompilerType::Wrapper) | None => return Err(execution),
        Some(ty) => ty,
    };

    Ok((
        Execution {
            executable: real_compiler,
            arguments: filtered_args,
            working_dir: execution.working_dir,
            environment: execution.environment,
        },
        inner_type,
    ))
}

/// Identify the wrapper by basename. Returns the static name string so
/// callers can branch on it without allocating.
fn detect_wrapper_name(executable: &Path) -> Option<&'static str> {
    let name = executable.file_stem()?.to_str()?;
    WRAPPER_NAMES.iter().copied().find(|&w| w == name)
}

/// Locate the real compiler in a wrapper invocation's argv and return the
/// surviving argv slice (compiler at index 0). Pure argv parsing -- does
/// not consult the recognizer; callers are responsible for validating that
/// the returned path is actually a compiler.
fn extract_real_compiler(wrapper_name: &str, args: &[String]) -> Option<(PathBuf, Vec<String>)> {
    match wrapper_name {
        // ccache, sccache, and icecc: argv[1] is the real compiler,
        // argv[2..] are its flags. In prefix usage they have no
        // wrapper-specific flags of their own that we need to skip.
        "ccache" | "sccache" | "icecc" => {
            let inner = args.get(1)?;
            Some((PathBuf::from(inner), args[1..].to_vec()))
        }
        // distcc accepts its own flags before the compiler name. Skip
        // them (consuming any flag values too) until we find a non-distcc
        // argv slot, which is the compiler.
        "distcc" => {
            let mut i = 1;
            while i < args.len() {
                let consumed = distcc_option_count(&args[i]);
                if consumed == 0 {
                    break;
                }
                i += consumed;
            }
            let inner = args.get(i)?;
            Some((PathBuf::from(inner), args[i..].to_vec()))
        }
        _ => None,
    }
}

/// Number of argv slots a distcc-specific option consumes (the flag plus
/// any value that follows). Zero means the argument is not a distcc option.
fn distcc_option_count(arg: &str) -> usize {
    match arg {
        "-j" | "--jobs" => 2,
        "-v" | "--verbose" | "-i" | "--show-hosts" | "--scan-avail" | "--show-principal" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_execution(args: Vec<&str>) -> Execution {
        Execution::from_strings(args[0], args, "/project", HashMap::new())
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_detect_wrapper_name() {
        let sut = |path_str| detect_wrapper_name(Path::new(path_str));

        assert_eq!(sut("/usr/bin/ccache"), Some("ccache"));
        assert_eq!(sut("/opt/distcc"), Some("distcc"));
        assert_eq!(sut("sccache"), Some("sccache"));
        assert_eq!(sut("/usr/lib/icecc/bin/icecc"), Some("icecc"));
        assert_eq!(sut("/usr/bin/gcc"), None);
        assert_eq!(sut("make"), None);
        // icerun launches arbitrary commands on the icecream cluster, not
        // compiler invocations -- it is deliberately not a launcher here.
        assert_eq!(sut("icerun"), None);
    }

    #[test]
    fn test_distcc_option_count() {
        assert_eq!(2, distcc_option_count("-j"));
        assert_eq!(2, distcc_option_count("--jobs"));
        assert_eq!(1, distcc_option_count("-v"));
        assert_eq!(1, distcc_option_count("--verbose"));
        assert_eq!(1, distcc_option_count("-i"));
        assert_eq!(1, distcc_option_count("--show-hosts"));
        assert_eq!(1, distcc_option_count("--scan-avail"));
        assert_eq!(1, distcc_option_count("--show-principal"));
        assert_eq!(0, distcc_option_count("-c"));
        assert_eq!(0, distcc_option_count("-Wall"));
        assert_eq!(0, distcc_option_count("--output"));
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_unwrap_extracts_real_compiler_for_valid_wrapper_calls() {
        let recognizer = CompilerRecognizer::new();
        let cases: Vec<(Vec<&str>, &str)> = vec![
            (vec!["ccache", "gcc", "-c", "main.c"], "gcc"),
            (vec!["/usr/bin/ccache", "gcc", "-c", "main.c"], "gcc"),
            (vec!["ccache", "/usr/bin/gcc", "-c", "main.c"], "/usr/bin/gcc"),
            (vec!["ccache", "clang", "-c", "main.c"], "clang"),
            (vec!["ccache", "/usr/bin/clang", "-c", "main.c"], "/usr/bin/clang"),
            (vec!["sccache", "gcc", "-c", "main.c"], "gcc"),
            (vec!["sccache", "clang", "-c", "main.c"], "clang"),
            (vec!["distcc", "-j", "4", "gcc", "-c", "main.c"], "gcc"),
            (vec!["distcc", "clang", "-c", "main.c"], "clang"),
            (vec!["icecc", "gcc", "-c", "main.c"], "gcc"),
            (vec!["/usr/bin/icecc", "clang", "-c", "main.c"], "clang"),
        ];

        for (args, expected_inner) in cases {
            let exec = create_execution(args.clone());
            let (inner, _ty) =
                unwrap(exec, &recognizer).unwrap_or_else(|_| panic!("unwrap should succeed for {:?}", args));
            assert_eq!(inner.executable, PathBuf::from(expected_inner));
        }
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_unwrap_rejects_non_wrapper_or_invalid_calls() {
        let recognizer = CompilerRecognizer::new();
        let cases: Vec<Vec<&str>> = vec![
            vec!["gcc", "-c", "main.c"],                     // not a wrapper at all
            vec!["make", "all"],                             // not a wrapper
            vec!["ccache"],                                  // wrapper without inner argv
            vec!["ccache", "make", "all"],                   // inner is not a compiler
            vec!["ccache", "distcc", "gcc", "-c", "main.c"], // wrapper-of-wrapper
            vec!["icecc"],                                   // launcher without inner argv
            vec!["icecc", "make", "all"],                    // inner is not a compiler
            vec!["icecc", "ccache", "gcc", "-c", "main.c"],  // launcher-of-launcher
        ];

        for args in cases {
            let exec = create_execution(args.clone());
            assert!(unwrap(exec, &recognizer).is_err(), "unwrap should reject {:?}", args);
        }
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_unwrap_preserves_working_dir_and_environment() {
        let recognizer = CompilerRecognizer::new();
        let mut env = HashMap::new();
        env.insert("CC", "gcc");
        let exec = Execution::from_strings(
            "/usr/bin/ccache",
            vec!["ccache", "gcc", "-c", "main.c"],
            "/custom/dir",
            env,
        );

        let (inner, ty) = unwrap(exec, &recognizer).expect("unwrap should succeed");

        assert_eq!(inner.working_dir, PathBuf::from("/custom/dir"));
        assert_eq!(inner.environment.get("CC"), Some(&"gcc".to_string()));
        assert_eq!(ty, CompilerType::Gcc);
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_unwrap_strips_distcc_flags_from_filtered_args() {
        let recognizer = CompilerRecognizer::new();
        let exec = Execution::from_strings(
            "/usr/bin/distcc",
            vec!["distcc", "-j", "4", "gcc", "-c", "main.c", "-o", "main.o"],
            "/project",
            HashMap::new(),
        );

        let (inner, _ty) = unwrap(exec, &recognizer).expect("unwrap should succeed");

        assert_eq!(inner.executable, PathBuf::from("gcc"));
        assert_eq!(inner.arguments, vec!["gcc", "-c", "main.c", "-o", "main.o"]);
    }
}

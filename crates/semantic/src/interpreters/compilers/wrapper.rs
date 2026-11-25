// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiler wrapper handling for ccache, distcc, sccache, and icecc.
//!
//! Wrappers sit between the build system and the real compiler:
//! `ccache gcc -c main.c`. The job here is small: detect the wrapper by
//! basename, locate the real compiler in argv (skipping wrapper-specific
//! flags like `distcc -j 4`), and report how many argv tokens belong to the
//! launcher. The caller (`CompilerInterpreter::recognize`) keeps the launcher
//! as a fact about the call and dispatches on the inner compiler.
//!
//! This module is the authority on what counts as a wrapper:
//! [`WRAPPER_NAMES`] is shared with `compiler_recognition`, which consults
//! it to skip the `--version` probe for a wrapper's own basename (the
//! recognizer's regex pattern itself comes from `RECOGNITION_PATTERNS`,
//! generated from each wrapper file's own `recognize` entry). Both
//! `WRAPPER_NAMES` and `WRAPPER_OPTIONS` are generated from
//! `crates/semantic/compilers/*.yaml` (the `type: wrapper` files) -- see
//! `build-support/compilers-codegen`.

use super::compiler_recognition::CompilerRecognizer;
use super::identity::CompilerType;
use intercept::Execution;

use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/wrappers.rs"));

/// Find the launcher in `execution`: how many argv tokens come before the
/// inner compiler, and the inner compiler's recognized [`CompilerType`] so the
/// caller can dispatch without re-running the recognizer.
///
/// Returns `None` when `execution` is not a wrapper invocation we recognize,
/// or when the inner argv names no real compiler: missing inner argument,
/// inner not a compiler, or wrapper-of-wrapper (e.g. `ccache distcc gcc`).
pub(super) fn unwrap(
    execution: &Execution,
    recognizer: &CompilerRecognizer,
) -> Option<(usize, CompilerType)> {
    let wrapper_name = detect_wrapper_name(&execution.executable)?;
    let launcher_len = launcher_len(wrapper_name, &execution.arguments)?;

    // The inner argv must name a real compiler; reject wrapper-of-wrapper
    // (which would otherwise loop) and unknown executables.
    match recognizer.recognize(Path::new(&execution.arguments[launcher_len])) {
        Some(CompilerType::Wrapper) | None => None,
        Some(inner_type) => Some((launcher_len, inner_type)),
    }
}

/// Identify the wrapper by basename. Returns the static name string so
/// callers can branch on it without allocating.
fn detect_wrapper_name(executable: &Path) -> Option<&'static str> {
    let name = executable.file_stem()?.to_str()?;
    WRAPPER_NAMES.iter().copied().find(|&w| w == name)
}

/// Count the argv tokens before the real compiler in a wrapper invocation:
/// the wrapper itself and its own options. The compiler is the token at that
/// index. Pure argv parsing -- does not consult the recognizer; callers are
/// responsible for validating that the token is actually a compiler.
///
/// Generic across every wrapper: skip `wrapper_name`'s own options (each
/// consuming `1 + arity` argv slots), then the next slot is the real
/// compiler. Most wrappers (ccache, sccache, icecc) have no options of
/// their own, so the loop body never matches and the count stays 1 -- argv[1]
/// is the compiler.
fn launcher_len(wrapper_name: &str, args: &[String]) -> Option<usize> {
    let opts = wrapper_options(wrapper_name);
    let mut i = 1;
    while i < args.len() {
        match opts.iter().find(|(flag, _)| *flag == args[i]) {
            Some((_, arity)) => i += 1 + *arity as usize,
            None => break,
        }
    }
    (i < args.len()).then_some(i)
}

/// The `(option, arity)` pairs `wrapper_name` accepts before its inner
/// compiler argv, or an empty slice if it declares none.
fn wrapper_options(wrapper_name: &str) -> &'static [(&'static str, u32)] {
    WRAPPER_OPTIONS.iter().find(|(name, _)| *name == wrapper_name).map(|(_, opts)| *opts).unwrap_or(&[])
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
    fn test_distcc_options_have_expected_arity() {
        let opts = wrapper_options("distcc");
        let cases: &[(&str, u32)] = &[
            ("-j", 1),
            ("--jobs", 1),
            ("-v", 0),
            ("--verbose", 0),
            ("-i", 0),
            ("--show-hosts", 0),
            ("--scan-avail", 0),
            ("--show-principal", 0),
        ];
        for (flag, expected_arity) in cases {
            let arity = opts.iter().find(|(f, _)| f == flag).map(|(_, a)| *a);
            assert_eq!(arity, Some(*expected_arity), "flag: {}", flag);
        }

        // Unknown flags (including compiler-own flags like -c/-Wall/--output
        // that distcc never claims) are not in the table.
        for unknown in ["-c", "-Wall", "--output"] {
            assert!(opts.iter().find(|(f, _)| *f == unknown).is_none(), "flag: {}", unknown);
        }
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
            let (len, _ty) =
                unwrap(&exec, &recognizer).unwrap_or_else(|| panic!("unwrap should succeed for {:?}", args));
            assert_eq!(exec.arguments[len], expected_inner, "case {args:?}");
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
            assert!(unwrap(&exec, &recognizer).is_none(), "unwrap should reject {:?}", args);
        }
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_unwrap_returns_the_inner_compiler_type() {
        let recognizer = CompilerRecognizer::new();
        let exec = Execution::from_strings(
            "/usr/bin/ccache",
            vec!["ccache", "gcc", "-c", "main.c"],
            "/custom/dir",
            HashMap::new(),
        );

        let (len, ty) = unwrap(&exec, &recognizer).expect("unwrap should succeed");

        assert_eq!(len, 1);
        assert_eq!(ty, CompilerType::compiler("gcc"));
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_unwrap_counts_the_launcher_options() {
        let recognizer = CompilerRecognizer::new();
        let exec = Execution::from_strings(
            "/usr/bin/distcc",
            vec!["distcc", "-j", "4", "gcc", "-c", "main.c", "-o", "main.o"],
            "/project",
            HashMap::new(),
        );

        let (len, _ty) = unwrap(&exec, &recognizer).expect("unwrap should succeed");

        assert_eq!(len, 3);
        assert_eq!(exec.arguments[len..], ["gcc", "-c", "main.c", "-o", "main.o"]);
    }
}

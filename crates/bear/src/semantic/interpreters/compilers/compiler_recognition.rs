// SPDX-License-Identifier: GPL-3.0-or-later

//! Unified compiler recognition using regex patterns.
//!
//! This module provides a consolidated approach to recognizing compiler executables
//! using regular expressions instead of separate hard-coded lists and pattern
//! matching functions for each compiler.

use super::probe::{CompilerProbe, default_probe};
use super::wrapper::WRAPPER_NAMES;
use crate::config::{Compiler, CompilerType};
use regex_lite::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Basenames whose underlying toolchain cannot be inferred from the name
/// alone -- `cc` and `c++` resolve to GCC on most Linuxes but Clang on
/// FreeBSD/OpenBSD/NetBSD/DragonFly and macOS. `CC` is the HPE Cray PrgEnv
/// wrapper: it drives whichever compiler module the loaded programming
/// environment selects (CCE, GCC, or another vendor's compiler), so the
/// basename alone is ambiguous the same way.
///
/// The probe is the sole classifier for these names: there is no regex
/// fallback (gcc.yaml and cray_cc.yaml deliberately omit them). When the
/// probe declines (timeout, unrecognizable output, spawn failure),
/// `recognize` returns `None` rather than guessing -- a missing entry is
/// visible and debuggable, whereas a wrongly-classified entry corrupts
/// the compilation database silently via mismatched flag-arity tables.
const AMBIGUOUS_NAMES: &[&str] = &["cc", "c++", "CC"];

/// Recognizes the compiler type for an executable path, using a layered
/// strategy:
///
/// 1. **Hint lookup** — user-supplied [`Compiler`] entries (canonicalized)
///    are checked first; a hit short-circuits both probe and regex.
/// 2. **Probe** — for ambiguous basenames (`cc`, `c++`, `CC`) the binary is
///    invoked with `--version` and classified by signature. Memoization
///    of probe results lives in the probe itself (see
///    [`super::probe::CachingProbe`]); the recognizer only owns the
///    dispatch policy.
/// 3. **Regex fallback** — filename is matched against patterns generated
///    from `interpreters/*.yaml`. Note that the ambiguous names above are
///    intentionally omitted from the regex; if the probe declines,
///    recognition returns `None` rather than guessing.
pub struct CompilerRecognizer {
    patterns: Vec<(CompilerType, Regex)>,
    hints: HashMap<PathBuf, CompilerType>,
    probe: Box<dyn CompilerProbe>,
}

impl CompilerRecognizer {
    /// Creates a new compiler recognizer with default patterns and the
    /// platform-default probe ([`super::probe::default_probe`] -- the real
    /// `--version` probe on Unix, a no-op on Windows where compiler
    /// basenames are unambiguous).
    pub fn new() -> Self {
        Self::with_probe(&[], default_probe())
    }

    /// Creates a new compiler recognizer with configuration-based hints.
    ///
    /// Uses the platform-default probe; user hints are consulted first and
    /// short-circuit the probe regardless of platform.
    ///
    /// # Arguments
    ///
    /// * `compilers` - Slice of compiler configurations with optional type hints
    pub fn new_with_config(compilers: &[Compiler]) -> Self {
        Self::with_probe(compilers, default_probe())
    }

    /// Creates a recognizer with an injectable probe. Used by tests to swap
    /// in a fake probe that does not fork+exec.
    pub(crate) fn with_probe(compilers: &[Compiler], probe: Box<dyn CompilerProbe>) -> Self {
        Self { patterns: DEFAULT_PATTERNS.clone(), hints: Self::build_hints_map(compilers), probe }
    }

    /// Recognizes the compiler type from an executable path.
    ///
    /// Order: configured hints, then `--version` probe (only for the
    /// ambiguous basename set), then regex.
    ///
    /// # Arguments
    ///
    /// * `executable_path` - The path to the executable (can be relative or absolute)
    ///
    /// # Returns
    ///
    /// `Some(CompilerType)` if the executable is recognized, `None` otherwise
    pub fn recognize(&self, executable_path: &Path) -> Option<CompilerType> {
        // 1. Check configured hints first (by canonical path matching).
        //    The user override always wins; it also short-circuits the probe.
        if let Some(hint_type) = self.lookup_hint(executable_path) {
            return Some(hint_type);
        }

        // 2. For known-ambiguous basenames, run the --version probe. The
        //    probe is what distinguishes BSD/macOS `cc` (Clang) from Linux
        //    `cc` (GCC). Caching of probe results lives inside the probe
        //    itself ([`super::probe::CachingProbe`]) so the cost is at most
        //    one fork+exec per unique compiler path per process.
        let filename = executable_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if AMBIGUOUS_NAMES.contains(&filename)
            && let Some(t) = self.probe_canonical(executable_path)
        {
            return Some(t);
        }

        // 3. Fall back to regex-based recognition.
        self.recognize_by_regex(executable_path)
    }

    /// Looks up a hint for the given executable path.
    ///
    /// Tries both the original path and its canonicalized version.
    fn lookup_hint(&self, executable_path: &Path) -> Option<CompilerType> {
        // Try original path first
        if let Some(&compiler_type) = self.hints.get(executable_path) {
            return Some(compiler_type);
        }

        // Try canonicalized path
        if let Ok(canonical_path) = executable_path.canonicalize()
            && let Some(&compiler_type) = self.hints.get(&canonical_path)
        {
            return Some(compiler_type);
        }

        None
    }

    /// Canonicalize `executable_path` and ask the probe to classify it.
    ///
    /// Canonicalization happens here (not in the probe) for two reasons:
    /// the wrapper guard below relies on the canonical basename, and
    /// canonicalizing before the cache key in
    /// [`super::probe::CachingProbe`] collapses different argv spellings
    /// of the same compiler into one cache entry.
    ///
    /// Wrapper safety: if the canonical path resolves to a wrapper
    /// (ccache/distcc/sccache, e.g. via a symlink farm where `cc` ->
    /// `/usr/lib/ccache/ccache`), do not probe -- the wrapper's
    /// `--version` reports its own banner, not the compiler's, and we
    /// want the regex layer to classify it as `CompilerType::Wrapper`
    /// so the wrapper interpreter unwraps it.
    fn probe_canonical(&self, executable_path: &Path) -> Option<CompilerType> {
        let key = executable_path.canonicalize().unwrap_or_else(|_| executable_path.to_path_buf());

        if let Some(name) = key.file_name().and_then(|n| n.to_str())
            && WRAPPER_NAMES.contains(&name)
        {
            return None;
        }

        self.probe.probe(&key)
    }

    /// Internal regex-based recognition.
    ///
    /// Ignores the directory path and only looks at the filename to
    /// determine the compiler type using [`DEFAULT_PATTERNS`].
    fn recognize_by_regex(&self, executable_path: &Path) -> Option<CompilerType> {
        let filename = executable_path.file_name()?.to_str()?;

        // Check each compiler pattern
        for (compiler_type, pattern) in &self.patterns {
            if pattern.is_match(filename) {
                return Some(*compiler_type);
            }
        }

        None
    }

    /// Creates a hint lookup table from compiler configuration.
    ///
    /// This method processes a slice of [`Compiler`] configurations and builds a mapping
    /// from filesystem paths to compiler types. This allows for explicit compiler type
    /// specification that overrides pattern-based recognition.
    ///
    /// # Arguments
    ///
    /// * `compilers` - A slice of [`Compiler`] configurations from which to extract hints
    ///
    /// # Returns
    ///
    /// A [`HashMap`] mapping canonicalized [`PathBuf`]s to their corresponding [`CompilerType`]s.
    /// All compilers that are not marked as `ignore = true` will be included in the mapping.
    ///
    /// # Compiler Type Resolution
    ///
    /// For each non-ignored compiler, the compiler type is determined as follows:
    /// 1. **Explicit `as_` field**: If the compiler has an `as_` field specified, that type is used
    /// 2. **Pattern matching**: If `as_` is `None`, the filename is matched against default patterns
    ///    (GCC, Clang, Fortran, Intel Fortran, Cray Fortran)
    /// 3. **Fallback**: If no pattern matches, defaults to [`CompilerType::Gcc`]
    ///
    /// # Path Canonicalization
    ///
    /// The method attempts to canonicalize each compiler path using [`PathBuf::canonicalize()`].
    /// If canonicalization fails (e.g., due to the path not existing), the original path
    /// is used instead. This helps with matching paths that may be specified differently
    /// but refer to the same executable.
    ///
    /// # Examples
    ///
    /// Given a configuration like:
    /// ```yaml
    /// compilers:
    ///   - path: /usr/bin/my-custom-gcc
    ///     as: gcc
    ///   - path: /opt/llvm/bin/clang++        # No 'as' field - will be guessed as Clang
    ///   - path: /usr/bin/unknown-compiler    # No 'as' field - will default to GCC
    ///   - path: /usr/bin/ignored-compiler
    ///     ignore: true
    /// ```
    ///
    /// This method would return a mapping containing entries for the first three compilers
    /// but exclude the fourth due to the `ignore` flag. The second compiler would be
    /// recognized as Clang through pattern matching, and the third would default to GCC.
    fn build_hints_map(compilers: &[Compiler]) -> HashMap<PathBuf, CompilerType> {
        let mut hints = HashMap::new();

        for compiler in compilers {
            // Skip ignored compilers
            if compiler.ignore {
                continue;
            }

            // Try to canonicalize the path for better matching
            let canonical_path = compiler.path.canonicalize().unwrap_or_else(|_| compiler.path.clone());

            let compiler_type = if let Some(as_type) = compiler.as_ {
                // Use explicitly configured compiler type
                as_type
            } else {
                // Guess compiler type using default patterns
                let filename = compiler.path.file_name().and_then(|name| name.to_str()).unwrap_or("");

                let guessed_type = DEFAULT_PATTERNS
                    .iter()
                    .find(|(_, pattern)| pattern.is_match(filename))
                    .map(|(compiler_type, _)| *compiler_type);

                // Fall back to GCC if no pattern matches
                guessed_type.unwrap_or(CompilerType::Gcc)
            };

            hints.insert(canonical_path, compiler_type);
        }

        hints
    }
}

impl Default for CompilerRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

// ----- Pattern infrastructure -----------------------------------------
//
// Default regex patterns and the helpers that build them. These are
// implementation details of CompilerRecognizer and are not part of its
// public surface. The patterns themselves are generated at build time
// from the YAML files under `bear/interpreters/`.

// Generated recognition pattern data from flags/*.yaml.
include!(concat!(env!("OUT_DIR"), "/recognition.rs"));

/// Compile-time initialized default regex patterns for compiler recognition.
///
/// Built from YAML-defined `recognize` entries plus a hand-written Wrapper pattern.
/// Each entry maps a `CompilerType` to a regex that matches executable filenames,
/// supporting cross-compilation prefixes, version suffixes, and `.exe` extensions.
static DEFAULT_PATTERNS: LazyLock<Vec<(CompilerType, Regex)>> = LazyLock::new(|| {
    let mut patterns = Vec::new();

    // Build patterns from generated YAML data
    for &(type_str, executables, cross_compilation, versioned) in RECOGNITION_PATTERNS {
        let compiler_type = parse_compiler_type(type_str);
        let regex = create_compiler_regex(executables, cross_compilation, versioned);
        patterns.push((compiler_type, regex));
    }

    // Wrapper pattern stays hand-written (not YAML-driven)
    patterns.push((CompilerType::Wrapper, create_compiler_regex(WRAPPER_NAMES, false, false)));

    patterns
});

/// Map a YAML `type` string to a `CompilerType` variant.
fn parse_compiler_type(type_str: &str) -> CompilerType {
    match type_str {
        "gcc" => CompilerType::Gcc,
        "clang" => CompilerType::Clang,
        "flang" => CompilerType::Flang,
        "intel_fortran" => CompilerType::IntelFortran,
        "cray_fortran" => CompilerType::CrayFortran,
        "cuda" => CompilerType::Cuda,
        "msvc" => CompilerType::Msvc,
        "clang_cl" => CompilerType::ClangCl,
        "intel_cc" => CompilerType::IntelCc,
        "nvidia_hpc" => CompilerType::NvidiaHpc,
        "armclang" => CompilerType::Armclang,
        "ibm_xl" => CompilerType::IbmXl,
        "vala" => CompilerType::Vala,
        "mpi" => CompilerType::Mpi,
        "cray_cc" => CompilerType::CrayCc,
        "qnx" => CompilerType::Qnx,
        other => panic!("Unknown compiler type in YAML: '{}'", other),
    }
}

/// Build a regex that matches any of the given `executables`, with optional
/// cross-compilation prefix and version suffix support, plus `.exe` extension.
fn create_compiler_regex(executables: &[&str], cross_compilation: bool, versioned: bool) -> Regex {
    // Escape for regex (handles '+' in names like "c++", "clang++")
    let escaped: Vec<String> = executables.iter().map(|n| escape_executable(n)).collect();
    let alternation = escaped.join("|");

    let base = if cross_compilation {
        format!(r"(?:[^/]*-)?(?:{})", alternation)
    } else {
        format!(r"(?:{})", alternation)
    };

    let with_version =
        if versioned { format!(r"{}(?:[-_]?([0-9]+(?:[._-][0-9a-zA-Z]+)*))?", base) } else { base };

    // On Windows, executable names are case-insensitive (CL.EXE, cl.exe, Cl.exe)
    let case_flag = if cfg!(windows) { "(?i)" } else { "" };
    let full_pattern = format!(r"^{}{}(?:\.exe)?$", case_flag, with_version);
    Regex::new(&full_pattern).unwrap_or_else(|_| panic!("Invalid regex pattern: {}", full_pattern))
}

/// Escape regex metacharacters that may appear in compiler executable names.
///
/// The only metacharacter that actually appears in the YAML-defined executable
/// names is `+` (e.g. `c++`, `g++`, `clang++`). `regex-lite` does not expose a
/// public `escape` helper, so we provide a minimal local one.
fn escape_executable(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c == '+' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::probe::NoProbe;
    use super::*;
    use std::path::Path;

    fn path(s: &str) -> &Path {
        Path::new(s)
    }

    /// Recognizer with the probe disabled. Use this for tests that exercise
    /// the regex/hint layer in isolation. Tests that need to verify the
    /// probe should construct a recognizer with a `FakeProbe`.
    fn no_probe_recognizer() -> CompilerRecognizer {
        CompilerRecognizer::with_probe(&[], Box::new(NoProbe))
    }

    #[test]
    fn test_gcc_recognition() {
        // Pure regex behavior. The bare names `cc` and `c++` are
        // intentionally absent from the gcc.yaml regex: they are
        // ambiguous (Linux=GCC, BSDs/macOS=Clang) and dispatch is owned
        // by the probe. Tests for those names live in the probe_* group.
        let recognizer = no_probe_recognizer();

        // Basic GCC names
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++")), Some(CompilerType::Gcc));

        // Cross-compilation variants
        assert_eq!(recognizer.recognize(path("arm-linux-gnueabi-gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("aarch64-linux-gnu-g++")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("x86_64-w64-mingw32-gcc")), Some(CompilerType::Gcc));

        // Versioned variants
        assert_eq!(recognizer.recognize(path("gcc-9")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++-11")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("gcc-11.2")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("gcc9")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++11")), Some(CompilerType::Gcc));

        // With full paths
        assert_eq!(recognizer.recognize(path("/usr/bin/gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("/opt/gcc/bin/g++")), Some(CompilerType::Gcc));
    }

    #[test]
    fn test_clang_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Basic Clang names
        assert_eq!(recognizer.recognize(path("clang")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang++")), Some(CompilerType::Clang));

        // Cross-compilation variants
        assert_eq!(recognizer.recognize(path("aarch64-linux-gnu-clang")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("arm-linux-gnueabi-clang++")), Some(CompilerType::Clang));

        // Versioned variants
        assert_eq!(recognizer.recognize(path("clang-15")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang++-16")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang15")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang++16")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang-15.0")), Some(CompilerType::Clang));

        // With full paths
        assert_eq!(recognizer.recognize(path("/usr/bin/clang")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("/opt/llvm/bin/clang++")), Some(CompilerType::Clang));
    }

    #[test]
    fn test_windows_exe_extensions() {
        let recognizer = CompilerRecognizer::new();

        // GCC with .exe extensions. (`cc.exe`/`c++.exe` are intentionally
        // absent: those names are ambiguous and the probe owns dispatch
        // for them; the regex returns no match.)
        assert_eq!(recognizer.recognize(path("gcc.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++.exe")), Some(CompilerType::Gcc));

        // Cross-compilation variants with .exe
        assert_eq!(recognizer.recognize(path("arm-linux-gnueabi-gcc.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("x86_64-w64-mingw32-g++.exe")), Some(CompilerType::Gcc));

        // Versioned variants with .exe
        assert_eq!(recognizer.recognize(path("gcc-9.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++-11.2.exe")), Some(CompilerType::Gcc));

        // Clang with .exe extensions
        assert_eq!(recognizer.recognize(path("clang.exe")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang++.exe")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang-15.exe")), Some(CompilerType::Clang));

        // Fortran with .exe extensions
        assert_eq!(recognizer.recognize(path("gfortran.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("flang.exe")), Some(CompilerType::Flang));
        assert_eq!(recognizer.recognize(path("f95.exe")), Some(CompilerType::Gcc));

        // Intel Fortran with .exe extensions
        assert_eq!(recognizer.recognize(path("ifort.exe")), Some(CompilerType::IntelFortran));
        assert_eq!(recognizer.recognize(path("ifx.exe")), Some(CompilerType::IntelFortran));

        // Cray Fortran with .exe extensions
        assert_eq!(recognizer.recognize(path("crayftn.exe")), Some(CompilerType::CrayFortran));
        assert_eq!(recognizer.recognize(path("ftn.exe")), Some(CompilerType::CrayFortran));

        // CUDA with .exe extensions
        assert_eq!(recognizer.recognize(path("nvcc.exe")), Some(CompilerType::Cuda));

        // Wrapper tools with .exe extensions
        assert_eq!(recognizer.recognize(path("ccache.exe")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("distcc.exe")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("sccache.exe")), Some(CompilerType::Wrapper));
    }

    #[test]
    fn test_windows_paths_with_exe() {
        let recognizer = CompilerRecognizer::new();

        // Simple Unix-style paths with .exe (should work cross-platform)
        assert_eq!(recognizer.recognize(path("/mingw64/bin/gcc.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("/usr/bin/clang.exe")), Some(CompilerType::Clang));
    }

    #[test]
    fn test_fortran_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Basic Fortran names
        assert_eq!(recognizer.recognize(path("gfortran")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("f95")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("flang")), Some(CompilerType::Flang));
        assert_eq!(recognizer.recognize(path("flang-new")), Some(CompilerType::Flang));

        // Cross-compilation variants
        assert_eq!(recognizer.recognize(path("arm-linux-gnueabi-gfortran")), Some(CompilerType::Gcc));

        // Versioned variants
        assert_eq!(recognizer.recognize(path("gfortran-11")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("gfortran11")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("f95-4.8")), Some(CompilerType::Gcc));
    }

    #[test]
    fn test_intel_fortran_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Intel Fortran names
        assert_eq!(recognizer.recognize(path("ifort")), Some(CompilerType::IntelFortran));
        assert_eq!(recognizer.recognize(path("ifx")), Some(CompilerType::IntelFortran));

        // Versioned variants
        assert_eq!(recognizer.recognize(path("ifort-2021")), Some(CompilerType::IntelFortran));
        assert_eq!(recognizer.recognize(path("ifx-2023")), Some(CompilerType::IntelFortran));
    }

    #[test]
    fn test_cray_fortran_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Cray Fortran names
        assert_eq!(recognizer.recognize(path("crayftn")), Some(CompilerType::CrayFortran));
        assert_eq!(recognizer.recognize(path("ftn")), Some(CompilerType::CrayFortran));
    }

    #[test]
    fn test_unrecognized_executables() {
        let recognizer = CompilerRecognizer::new();

        // Should not recognize these
        assert_eq!(recognizer.recognize(path("unknown-compiler")), None);
        assert_eq!(recognizer.recognize(path("make")), None);
        assert_eq!(recognizer.recognize(path("cmake")), None);
        assert_eq!(recognizer.recognize(path("rustc")), None);
        assert_eq!(recognizer.recognize(path("javac")), None);
    }

    #[test]
    fn test_path_independence() {
        let recognizer = CompilerRecognizer::new();

        // The directory path should not matter, only the filename
        let paths =
            vec!["gcc", "./gcc", "/usr/bin/gcc", "/opt/custom/path/gcc", "../../../../some/deep/path/gcc"];

        for path_str in paths {
            assert_eq!(
                recognizer.recognize(path(path_str)),
                Some(CompilerType::Gcc),
                "Failed for path: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_recognize_with_config_hints() {
        use crate::config::Compiler;
        use std::path::PathBuf;

        // Create test compiler configurations with hints
        let compilers = vec![
            Compiler {
                path: PathBuf::from("custom-gcc-wrapper"),
                as_: Some(CompilerType::Gcc),
                ignore: false,
            },
            Compiler {
                path: PathBuf::from("weird-clang-name"),
                as_: Some(CompilerType::Clang),
                ignore: false,
            },
        ];

        let recognizer = CompilerRecognizer::new_with_config(&compilers);

        // Configured hints take priority
        assert_eq!(recognizer.recognize(path("custom-gcc-wrapper")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("weird-clang-name")), Some(CompilerType::Clang));

        // Regex detection still works for non-configured compilers
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("unknown-compiler")), None);
    }

    #[test]
    fn test_is_compiler_type() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("clang")), Some(CompilerType::Clang));
        assert_ne!(recognizer.recognize(path("gcc")), Some(CompilerType::Clang));
        assert_ne!(recognizer.recognize(path("clang")), Some(CompilerType::Gcc));
    }

    #[test]
    fn test_empty_config() {
        // Test that recognizer with empty config works the same as new()
        let recognizer_new = CompilerRecognizer::new();
        let recognizer_empty_config = CompilerRecognizer::new_with_config(&[]);

        assert_eq!(recognizer_new.recognize(path("gcc")), recognizer_empty_config.recognize(path("gcc")));
        assert_eq!(recognizer_new.recognize(path("clang")), recognizer_empty_config.recognize(path("clang")));
        assert_eq!(
            recognizer_new.recognize(path("unknown")),
            recognizer_empty_config.recognize(path("unknown"))
        );
    }

    #[test]
    fn test_gcc_internal_executables_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Test that GCC internal executables are recognized as GCC type
        assert_eq!(recognizer.recognize(path("cc1")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("cc1plus")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("cc1obj")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("cc1objplus")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("collect2")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("f951")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("lto1")), Some(CompilerType::Gcc));

        // Test with full paths
        assert_eq!(
            recognizer.recognize(path("/usr/libexec/gcc/x86_64-linux-gnu/11/cc1")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus")),
            Some(CompilerType::Gcc)
        );

        // Test that non-GCC internal executables are not matched by this pattern
        assert_eq!(recognizer.recognize(path("cc1foo")), None);
        assert_eq!(recognizer.recognize(path("foo-cc1")), None);
    }

    #[test]
    fn test_build_hints_map_improved_behavior() {
        use crate::config::Compiler;
        use std::path::PathBuf;

        // Create test compiler configurations with various scenarios
        let compilers = vec![
            // Compiler with explicit 'as' field - should use that type
            Compiler { path: PathBuf::from("custom-wrapper"), as_: Some(CompilerType::Clang), ignore: false },
            // Compiler without 'as' field but matches default pattern - should guess Clang
            Compiler { path: PathBuf::from("clang++"), as_: None, ignore: false },
            // Compiler without 'as' field and no pattern match - should fall back to GCC
            Compiler { path: PathBuf::from("unknown-compiler"), as_: None, ignore: false },
            // Ignored compiler - should not be included in hints
            Compiler { path: PathBuf::from("ignored-gcc"), as_: Some(CompilerType::Gcc), ignore: true },
            // Another compiler without 'as' field matching Fortran pattern
            Compiler { path: PathBuf::from("gfortran"), as_: None, ignore: false },
        ];

        let recognizer = CompilerRecognizer::new_with_config(&compilers);

        // Test explicit 'as' field is used
        assert_eq!(recognizer.recognize(path("custom-wrapper")), Some(CompilerType::Clang));

        // Test pattern matching works when 'as' is None
        assert_eq!(recognizer.recognize(path("clang++")), Some(CompilerType::Clang));

        // Test fallback to GCC when no pattern matches
        assert_eq!(recognizer.recognize(path("unknown-compiler")), Some(CompilerType::Gcc));

        // Test ignored compiler is not recognized via hints
        assert_eq!(
            recognizer.recognize(path("ignored-gcc")),
            Some(CompilerType::Gcc) // Should fall back to regex pattern, not hint
        );

        // Test Fortran pattern matching when 'as' is None
        assert_eq!(recognizer.recognize(path("gfortran")), Some(CompilerType::Gcc));
    }

    #[test]
    fn test_cuda_recognition() {
        let recognizer = CompilerRecognizer::default();

        // Test basic CUDA compiler recognition
        assert_eq!(recognizer.recognize(path("nvcc")), Some(CompilerType::Cuda));

        // Test versioned CUDA compiler
        assert_eq!(recognizer.recognize(path("nvcc-12.0")), Some(CompilerType::Cuda));

        // Test cross-compilation CUDA compiler
        assert_eq!(recognizer.recognize(path("aarch64-linux-gnu-nvcc")), Some(CompilerType::Cuda));

        // Test non-CUDA executables don't match
        // Note: fake-nvcc matches because it looks like a cross-compilation target
        assert_eq!(recognizer.recognize(path("nvcc-fake")), None); // Invalid suffix
        assert_eq!(recognizer.recognize(path("not-nvcc-at-all")), None);
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_wrapper_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Test wrapper recognition
        assert_eq!(recognizer.recognize(path("ccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("distcc")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("sccache")), Some(CompilerType::Wrapper));

        // Test with full paths
        assert_eq!(recognizer.recognize(path("/usr/bin/ccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("/opt/distcc/bin/distcc")), Some(CompilerType::Wrapper));

        // Test non-wrapper executables don't match
        assert_eq!(recognizer.recognize(path("ccache-fake")), None);
        assert_eq!(recognizer.recognize(path("fake-distcc")), None);
        assert_eq!(recognizer.recognize(path("not-sccache")), None);
    }

    #[test]
    fn test_version_capture_functionality() {
        // Test that the DEFAULT_PATTERNS contain regexes that can extract version numbers
        let recognizer = CompilerRecognizer::new();

        // Test basic dash-separated versions
        assert_eq!(recognizer.recognize(path("gcc-11")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++-9.3.0")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("clang-15")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("clang-12.1")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("gfortran-12")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("ifort-2023")), Some(CompilerType::IntelFortran));
        assert_eq!(recognizer.recognize(path("nvcc-11.8")), Some(CompilerType::Cuda));

        // Test underscore-separated versions
        assert_eq!(recognizer.recognize(path("gcc_11")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("clang_15.0.7")), Some(CompilerType::Clang));

        // Test that non-versioned compilers still work
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("clang")), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(path("gfortran")), Some(CompilerType::Gcc));

        // Test that wrapper executables don't have version patterns (as expected)
        assert_eq!(recognizer.recognize(path("ccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("ccache-1.0")), None); // No version pattern for wrappers

        // Verify behaviorally that GCC patterns include a versioned variant:
        // a numeric-suffixed name resolves to GCC, while a non-numeric suffix
        // (which the version sub-pattern rejects) does not.
        assert_eq!(recognizer.recognize(path("gcc-11")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("gcc-abc")), None);
    }

    #[test]
    fn test_msvc_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("cl")), Some(CompilerType::Msvc));
        assert_eq!(recognizer.recognize(path("cl.exe")), Some(CompilerType::Msvc));

        // Internal executables should be recognized as MSVC (then ignored by interpreter)
        assert_eq!(recognizer.recognize(path("c1")), Some(CompilerType::Msvc));
        assert_eq!(recognizer.recognize(path("c1xx")), Some(CompilerType::Msvc));
        assert_eq!(recognizer.recognize(path("c2")), Some(CompilerType::Msvc));
    }

    #[test]
    fn test_clang_cl_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("clang-cl")), Some(CompilerType::ClangCl));
        assert_eq!(recognizer.recognize(path("clang-cl.exe")), Some(CompilerType::ClangCl));
        assert_eq!(recognizer.recognize(path("clang-cl-17")), Some(CompilerType::ClangCl));
    }

    #[test]
    fn test_intel_cc_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("icx")), Some(CompilerType::IntelCc));
        assert_eq!(recognizer.recognize(path("icpx")), Some(CompilerType::IntelCc));
        assert_eq!(recognizer.recognize(path("icc")), Some(CompilerType::IntelCc));
        assert_eq!(recognizer.recognize(path("icpc")), Some(CompilerType::IntelCc));
        assert_eq!(recognizer.recognize(path("icx-2024")), Some(CompilerType::IntelCc));
    }

    #[test]
    fn test_nvidia_hpc_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("nvc")), Some(CompilerType::NvidiaHpc));
        assert_eq!(recognizer.recognize(path("nvc++")), Some(CompilerType::NvidiaHpc));
        assert_eq!(recognizer.recognize(path("nvfortran")), Some(CompilerType::NvidiaHpc));
        assert_eq!(recognizer.recognize(path("pgcc")), Some(CompilerType::NvidiaHpc));
        assert_eq!(recognizer.recognize(path("pgc++")), Some(CompilerType::NvidiaHpc));
        assert_eq!(recognizer.recognize(path("pgfortran")), Some(CompilerType::NvidiaHpc));
    }

    #[test]
    fn test_armclang_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("armclang")), Some(CompilerType::Armclang));
        assert_eq!(recognizer.recognize(path("armclang++")), Some(CompilerType::Armclang));
        assert_eq!(recognizer.recognize(path("armclang-14")), Some(CompilerType::Armclang));
    }

    // Requirements: recognition-cray-compilers
    #[test]
    fn test_cray_cc_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Case sensitivity note: on Unix the recognition regex is
        // case-sensitive, so "crayCC" only matches because it is listed as
        // its own literal alternative in cray_cc.yaml, not because "craycc"
        // case-folds to it. Both spellings must resolve independently.
        for name in ["craycc", "crayCC", "craycxx"] {
            assert_eq!(recognizer.recognize(path(name)), Some(CompilerType::CrayCc), "name: {}", name);
        }

        // Versioned variant
        assert_eq!(recognizer.recognize(path("craycc-17")), Some(CompilerType::CrayCc));
    }

    // Requirements: recognition-amd-compilers
    #[test]
    fn test_amd_compiler_recognition() {
        let recognizer = CompilerRecognizer::new();

        for name in ["amdclang", "amdclang++", "hipcc"] {
            assert_eq!(recognizer.recognize(path(name)), Some(CompilerType::Clang), "name: {}", name);
        }
        assert_eq!(recognizer.recognize(path("amdflang")), Some(CompilerType::Flang));

        // A GPU-arch reporting tool, not a compiler driver -- must not be recognized.
        assert_eq!(recognizer.recognize(path("amdgpu-arch")), None);
    }

    // Requirements: recognition-mpi-wrappers
    #[test]
    fn test_mpi_wrapper_recognition() {
        let recognizer = CompilerRecognizer::new();

        for name in ["mpicc", "mpicxx", "mpic++", "mpiCC", "mpifort", "mpif77", "mpif90"] {
            assert_eq!(recognizer.recognize(path(name)), Some(CompilerType::Mpi), "name: {}", name);
        }

        // Versioned variant
        assert_eq!(recognizer.recognize(path("mpicc-14")), Some(CompilerType::Mpi));

        // MPI launchers execute programs, they do not compile -- must not be recognized.
        assert_eq!(recognizer.recognize(path("mpirun")), None);
        assert_eq!(recognizer.recognize(path("mpiexec")), None);
    }

    // Requirements: recognition-mpi-wrappers
    #[test]
    fn test_intel_mpi_wrapper_recognition() {
        let recognizer = CompilerRecognizer::new();

        for name in ["mpiicc", "mpiicpc", "mpiicx", "mpiicpx"] {
            assert_eq!(recognizer.recognize(path(name)), Some(CompilerType::IntelCc), "name: {}", name);
        }
        for name in ["mpiifort", "mpiifx"] {
            assert_eq!(recognizer.recognize(path(name)), Some(CompilerType::IntelFortran), "name: {}", name);
        }
    }

    // Requirements: recognition-embedded-toolchains
    #[test]
    fn test_qnx_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("qcc")), Some(CompilerType::Qnx));
        assert_eq!(recognizer.recognize(path("q++")), Some(CompilerType::Qnx));

        // A name that merely shares the "q" prefix is not a QNX driver.
        assert_eq!(recognizer.recognize(path("qnxcc")), None);
        assert_eq!(recognizer.recognize(path("qcc-fake")), None);
    }

    #[test]
    fn test_ibm_xl_recognition() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("ibm-clang")), Some(CompilerType::IbmXl));
        assert_eq!(recognizer.recognize(path("ibm-clang++")), Some(CompilerType::IbmXl));
        assert_eq!(recognizer.recognize(path("xlclang")), Some(CompilerType::IbmXl));
        assert_eq!(recognizer.recognize(path("xlclang++")), Some(CompilerType::IbmXl));
    }

    #[test]
    fn test_case_sensitivity_behavior() {
        let recognizer = CompilerRecognizer::new();

        // On Windows, these should match (case-insensitive regex)
        // On Unix, these should NOT match (case-sensitive regex)
        let upper_gcc = recognizer.recognize(path("GCC"));
        let upper_clang = recognizer.recognize(path("CLANG"));
        let mixed_gcc = recognizer.recognize(path("Gcc"));

        if cfg!(windows) {
            assert_eq!(upper_gcc, Some(CompilerType::Gcc));
            assert_eq!(upper_clang, Some(CompilerType::Clang));
            assert_eq!(mixed_gcc, Some(CompilerType::Gcc));
        } else {
            assert_eq!(upper_gcc, None);
            assert_eq!(upper_clang, None);
            assert_eq!(mixed_gcc, None);
        }
    }

    #[test]
    fn test_exe_extension_case_on_windows() {
        let recognizer = CompilerRecognizer::new();

        // On Windows, .EXE should also match due to case-insensitive regex
        let upper_exe = recognizer.recognize(path("gcc.EXE"));

        if cfg!(windows) {
            assert_eq!(upper_exe, Some(CompilerType::Gcc));
        } else {
            assert_eq!(upper_exe, None);
        }
    }

    // ----- Probe dispatch tests -----------------------------------------

    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Probe that returns canned answers and counts calls.
    struct FakeProbe {
        answers: StdMutex<HashMap<PathBuf, CompilerType>>,
        calls: AtomicUsize,
    }

    impl FakeProbe {
        fn new() -> Self {
            Self { answers: StdMutex::new(HashMap::new()), calls: AtomicUsize::new(0) }
        }

        fn answer(self, p: &str, t: CompilerType) -> Self {
            self.answers.lock().unwrap().insert(PathBuf::from(p), t);
            self
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl super::super::probe::CompilerProbe for FakeProbe {
        fn probe(&self, p: &Path) -> Option<CompilerType> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.answers.lock().unwrap().get(p).copied()
        }
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_classifies_cc_as_clang_on_bsd_like_host() {
        // Simulates `/usr/bin/cc` resolving to Clang (FreeBSD, macOS).
        // The relative path "cc" canonicalizes to itself when the file does
        // not exist, so the probe key is the original PathBuf.
        let probe = Box::new(FakeProbe::new().answer("cc", CompilerType::Clang));
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::Clang));
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_classifies_cc_as_gcc_on_linux_like_host() {
        let probe = Box::new(FakeProbe::new().answer("cc", CompilerType::Gcc));
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::Gcc));
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_inconclusive_yields_not_recognized() {
        // The probe is the sole classifier for ambiguous names; there is
        // no regex fallback. When the probe returns None, recognition
        // returns None and the dispatcher will surface NotRecognized.
        // This matters because the previous "default to gcc" behavior
        // produced silently wrong entries on BSD/macOS hosts where cc is
        // Clang — exactly the bug this whole mechanism exists to fix.
        let probe = Box::new(FakeProbe::new());
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(path("cc")), None);
        assert_eq!(recognizer.recognize(path("c++")), None);
    }

    // Requirements: recognition-cray-compilers
    #[test]
    fn probe_classifies_cray_prgenv_cc_as_clang_via_cray_banner() {
        // Simulates the HPE Cray PrgEnv wrapper "CC" resolving to the CCE
        // Clang frontend under PrgEnv-cray. The Cray banner ("Cray clang
        // version ...") contains the substring "clang version", which
        // classify_version_output already recognizes -- no change to the
        // classifier itself is needed for this case.
        let probe = Box::new(FakeProbe::new().answer("CC", CompilerType::Clang));
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(path("CC")), Some(CompilerType::Clang));
    }

    // Requirements: recognition-cray-compilers
    #[test]
    fn probe_classifies_cray_prgenv_cc_as_gcc_via_fsf_banner() {
        // Simulates "CC" resolving to g++ under PrgEnv-gnu.
        let probe = Box::new(FakeProbe::new().answer("CC", CompilerType::Gcc));
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(path("CC")), Some(CompilerType::Gcc));
    }

    // Requirements: recognition-cray-compilers
    #[test]
    fn config_hint_beats_probe_and_suppresses_it_for_cray_prgenv_cc() {
        let compilers =
            vec![Compiler { path: PathBuf::from("CC"), as_: Some(CompilerType::CrayCc), ignore: false }];
        let probe = Box::new(FakeProbe::new().answer("CC", CompilerType::Clang));
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(&compilers, probe);

        assert_eq!(recognizer.recognize(path("CC")), Some(CompilerType::CrayCc));
        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 0, "hint must short-circuit the probe");
    }

    // Requirements: recognition-cray-compilers
    #[test]
    fn probe_inconclusive_for_cray_prgenv_cc_yields_not_recognized() {
        // A programming environment whose compiler prints a banner the
        // probe does not know (e.g. nvc++ under PrgEnv-nvidia) stays
        // unrecognized -- documented limitation in
        // recognition-cray-compilers.md.
        let probe = Box::new(FakeProbe::new());
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(path("CC")), None);
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn config_hint_beats_probe_and_suppresses_it() {
        // The user's compilers: entry must win, and the probe must not run
        // when a hint already classifies the path.
        let compilers =
            vec![Compiler { path: PathBuf::from("cc"), as_: Some(CompilerType::Gcc), ignore: false }];
        let probe = Box::new(FakeProbe::new().answer("cc", CompilerType::Clang));
        // Take a raw pointer to the FakeProbe so we can read the call count
        // after handing ownership to the recognizer. Safe because the
        // recognizer owns the box for the duration of the test.
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(&compilers, probe);

        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::Gcc));
        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 0, "hint must short-circuit the probe");
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn non_ambiguous_names_are_not_probed() {
        let probe = Box::new(FakeProbe::new());
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        // A handful of non-ambiguous names: each must take the regex path
        // without going through the probe at all.
        for name in &["gcc", "clang", "g++", "clang++", "gfortran", "icx", "nvcc"] {
            let _ = recognizer.recognize(path(name));
        }

        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 0, "only AMBIGUOUS_NAMES should ever reach the probe");
    }

    // Cache behavior is the responsibility of `super::super::probe::CachingProbe`
    // and is exercised in its own test module. The recognizer simply asks the
    // probe; whether the answer is fresh or memoized is opaque here.

    // Requirements: recognition-ambiguous-name-probe, recognition-compiler-launchers
    #[test]
    fn wrapper_basenames_are_never_probed_even_under_ambiguous_paths() {
        // ccache, distcc, sccache must reach the regex (which returns
        // CompilerType::Wrapper). Probing them would return the underlying
        // compiler's version and bypass wrapper unwrapping.
        let probe = Box::new(FakeProbe::new());
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        // The basename guard runs after canonicalization. ccache itself is
        // not in AMBIGUOUS_NAMES so it would never enter the probe path
        // anyway; this asserts the documented invariant explicitly.
        assert_eq!(recognizer.recognize(path("ccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("distcc")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("sccache")), Some(CompilerType::Wrapper));

        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 0);
    }

    // The recognizer canonicalizes ambiguous-name paths before delegating
    // to the (caching) probe. Two argv spellings of the same compiler --
    // e.g. reached through different symlinks -- must therefore collapse
    // to one inner-probe call. This invariant used to be exercised
    // implicitly by the now-removed probe_runs_at_most_once_per_canonical_path
    // test; with caching extracted into CachingProbe, we restore explicit
    // coverage by wiring CachingProbe over a counting probe and observing
    // the call count after asking the recognizer to classify two
    // symlinks pointing at the same target.
    //
    // Requirements: recognition-ambiguous-name-probe
    #[test]
    #[cfg(unix)]
    fn distinct_paths_canonicalizing_to_same_target_share_cache_entry() {
        use super::super::probe::CachingProbe;
        use std::os::unix::fs::symlink;
        use std::sync::Arc;

        struct CountingProbe {
            calls: Arc<AtomicUsize>,
        }
        impl super::super::probe::CompilerProbe for CountingProbe {
            fn probe(&self, _: &Path) -> Option<CompilerType> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Some(CompilerType::Clang)
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real-cc");
        std::fs::write(&real, b"").expect("write real-cc");

        let dir_a = dir.path().join("a");
        let dir_b = dir.path().join("b");
        std::fs::create_dir(&dir_a).expect("mkdir a");
        std::fs::create_dir(&dir_b).expect("mkdir b");
        let link_a = dir_a.join("cc");
        let link_b = dir_b.join("cc");
        symlink(&real, &link_a).expect("symlink a/cc -> real-cc");
        symlink(&real, &link_b).expect("symlink b/cc -> real-cc");

        let calls = Arc::new(AtomicUsize::new(0));
        let counting = CountingProbe { calls: Arc::clone(&calls) };
        let probe: Box<dyn super::super::probe::CompilerProbe> = Box::new(CachingProbe::new(counting));
        let recognizer = CompilerRecognizer::with_probe(&[], probe);

        assert_eq!(recognizer.recognize(&link_a), Some(CompilerType::Clang));
        assert_eq!(recognizer.recognize(&link_b), Some(CompilerType::Clang));

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "canonicalization in the recognizer must collapse symlinks before the cache lookup"
        );
    }
}

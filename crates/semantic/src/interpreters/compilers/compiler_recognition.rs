// SPDX-License-Identifier: GPL-3.0-or-later

//! Unified compiler recognition using regex patterns.
//!
//! This module provides a consolidated approach to recognizing compiler executables
//! using regular expressions instead of separate hard-coded lists and pattern
//! matching functions for each compiler.

use super::identity::{CompilerType, SpellingError};
use super::probe::{CompilerProbe, default_probe};
use super::wrapper::WRAPPER_NAMES;
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
/// 1. **Hint lookup** -- driver-supplied [`CompilerHints`] entries
///    (canonicalized) are checked first; a hit short-circuits both probe and
///    regex. A bare executable spelling also matches a configured entry by
///    filename, because config paths must exist on disk while builds often
///    spell only the name.
/// 2. **Probe** — for ambiguous basenames (`cc`, `c++`, `CC`) the binary is
///    invoked with `--version` and classified by signature. Memoization
///    of probe results lives in the probe itself (see
///    `CachingProbe`); the recognizer only owns the
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
    /// platform-default probe (`default_probe` -- the real
    /// `--version` probe on Unix, a no-op on Windows where compiler
    /// basenames are unambiguous).
    pub fn new() -> Self {
        Self::with_probe(CompilerHints::new(), default_probe())
    }

    /// Creates a new compiler recognizer with driver-supplied hints.
    ///
    /// Uses the platform-default probe; user hints are consulted first and
    /// short-circuit the probe regardless of platform.
    ///
    /// # Arguments
    ///
    /// * `hints` - The hint table built by [`CompilerHints`]
    pub fn new_with_hints(hints: CompilerHints) -> Self {
        Self::with_probe(hints, default_probe())
    }

    /// Creates a recognizer with an injectable probe. Used by tests to swap
    /// in a fake probe that does not fork+exec.
    pub(crate) fn with_probe(hints: CompilerHints, probe: Box<dyn CompilerProbe>) -> Self {
        Self { patterns: DEFAULT_PATTERNS.clone(), hints: hints.entries, probe }
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
        //    itself (`CachingProbe`) so the cost is at most
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
    /// Tries both the original path and its canonicalized version. The map
    /// also carries the configured paths' filenames as keys, so a bare
    /// spelling matches directly; a path spelling cannot collide with a
    /// filename key because [`Path`] equality is component-wise.
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
    /// Canonicalization happens here (not in the probe) so the cache key
    /// in `CachingProbe` collapses different argv
    /// spellings of the same compiler into one entry.
    ///
    /// A masquerade link (an ambiguous name canonicalizing to a wrapper,
    /// e.g. `/usr/lib/ccache/cc` -> `ccache`) is the exception: it
    /// answers `--version` in the name it was invoked by, passing the
    /// flag through to the underlying compiler, so it is probed -- and
    /// cached -- under the invoked path. The canonical key would collapse
    /// `cc` and `c++` onto the wrapper binary, whose answer depends on
    /// argv[0]. See docs/rationale/ambiguous-cc-version-probe.md.
    fn probe_canonical(&self, executable_path: &Path) -> Option<CompilerType> {
        let key = executable_path.canonicalize().unwrap_or_else(|_| executable_path.to_path_buf());

        if let Some(name) = key.file_name().and_then(|n| n.to_str())
            && WRAPPER_NAMES.contains(&name)
        {
            self.probe.probe(executable_path)
        } else {
            self.probe.probe(&key)
        }
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
}

impl Default for CompilerRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the hint lookup table a [`CompilerRecognizer`] consults first.
///
/// The driver adds one configured compiler at a time -- a path plus the
/// optional `as:` spelling the user wrote -- so the configuration schema
/// never crosses into this module. Insertion order is the configuration
/// order, which is what the first-wins rule for colliding filenames is
/// defined against.
#[derive(Clone, Debug, Default)]
pub struct CompilerHints {
    entries: HashMap<PathBuf, CompilerType>,
}

impl CompilerHints {
    /// An empty hint table: recognition falls back to probe and regex only.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves `spelling` and records the resulting hint for `path`.
    ///
    /// The entry contributes two keys: its canonicalized path, and its bare
    /// filename -- builds often invoke a configured compiler by name alone,
    /// and the filename key lets the hint reach those spellings. When two
    /// entries share a filename but disagree on the type, the first wins and
    /// a warning is logged.
    ///
    /// # Compiler type resolution
    ///
    /// 1. **Explicit spelling**: `Some(spelling)` resolves to that type, or
    ///    fails with [`SpellingError`] when it names no known compiler.
    /// 2. **Pattern matching**: `None` matches the filename against the
    ///    default recognition patterns.
    /// 3. **Fallback**: if no pattern matches, the `gcc` family. Unreachable
    ///    for a spelling the user actually wrote.
    ///
    /// # Path canonicalization
    ///
    /// `path` is canonicalized for the path key; when canonicalization fails
    /// (e.g. the path does not exist) the original is used. This matches
    /// paths that are spelled differently but name the same executable.
    pub fn add(&mut self, path: &Path, spelling: Option<&str>) -> Result<(), SpellingError> {
        let compiler_type = match spelling {
            Some(spelling) => spelling.parse::<CompilerType>()?,
            None => Self::guess_from_filename(path),
        };

        if let Some(name) = path.file_name() {
            match self.entries.entry(PathBuf::from(name)) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(compiler_type);
                }
                std::collections::hash_map::Entry::Occupied(entry) => {
                    if *entry.get() != compiler_type {
                        log::warn!(
                            "compiler config: '{}' shares the filename '{}' with an earlier \
                             entry of a different type; the earlier entry classifies bare \
                             invocations of that name",
                            path.display(),
                            name.to_string_lossy()
                        );
                    }
                }
            }
        }
        // Never collides with a filename key for validated config:
        // config paths must exist, so canonicalization yields an
        // absolute, multi-component path.
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.entries.insert(canonical_path, compiler_type);

        Ok(())
    }

    /// Resolves `spelling` without recording a hint.
    ///
    /// A configured entry marked `ignore: true` contributes no hint, but a
    /// misspelled `as:` in one must still fail the run, so the driver routes
    /// those entries through here.
    pub fn check(spelling: Option<&str>) -> Result<(), SpellingError> {
        match spelling {
            Some(spelling) => spelling.parse::<CompilerType>().map(|_| ()),
            None => Ok(()),
        }
    }

    /// Guesses a compiler type for an entry that named no `as:` spelling, by
    /// matching its filename against the default recognition patterns and
    /// falling back to the `gcc` family.
    fn guess_from_filename(path: &Path) -> CompilerType {
        let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or("");

        DEFAULT_PATTERNS
            .iter()
            .find(|(_, pattern)| pattern.is_match(filename))
            .map(|(compiler_type, _)| *compiler_type)
            .unwrap_or(CompilerType::compiler("gcc"))
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
/// Built entirely from YAML-defined `recognize` entries, including the four
/// compiler-launcher (wrapper) files: each contributes its own `(Wrapper,
/// regex)` pattern matching only its own executable name(s), which is
/// behaviorally equivalent to one combined regex since matching is a linear
/// scan. Each entry maps a `CompilerType` to a regex that matches executable
/// filenames, supporting cross-compilation prefixes, version suffixes, and
/// `.exe` extensions.
static DEFAULT_PATTERNS: LazyLock<Vec<(CompilerType, Regex)>> = LazyLock::new(|| {
    let mut patterns = Vec::new();

    // Build patterns from generated YAML data. The id column is trusted
    // generated data, so it maps to a `CompilerType` without validation:
    // `"wrapper"` is the launcher kind, every other value is a compiler id.
    for &(type_str, executables, cross_compilation, versioned, _description) in RECOGNITION_PATTERNS {
        let compiler_type = CompilerType::from_recognized_id(type_str);
        let regex = create_compiler_regex(executables, cross_compilation, versioned);
        patterns.push((compiler_type, regex));
    }

    patterns
});

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
/// The metacharacters that actually appear in the YAML-defined executable
/// names are `+` (e.g. `c++`, `g++`, `clang++`) and `.` (e.g. `emcc.py`).
/// An unescaped `.` would be a one-character wildcard, so a name like
/// `emccxpy` would falsely match the `emcc.py` pattern. `regex-lite` does
/// not expose a public `escape` helper, so we provide a minimal local one.
fn escape_executable(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c == '+' || c == '.' {
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

    /// Builds a hint table from `(path, spelling)` pairs, the way the driver
    /// does for the configured compilers it does not ignore.
    fn hints_for(entries: &[(&str, Option<&str>)]) -> CompilerHints {
        let mut hints = CompilerHints::new();
        for (path, spelling) in entries {
            hints.add(Path::new(path), *spelling).expect("test spellings must resolve");
        }
        hints
    }

    /// Asserts each `(name, expected)` case against `recognizer`, naming the
    /// case in the failure message so a table failure says which row broke.
    fn assert_cases(recognizer: &CompilerRecognizer, cases: &[(&str, Option<CompilerType>)]) {
        for (name, expected) in cases {
            assert_eq!(recognizer.recognize(path(name)), *expected, "name: {}", name);
        }
    }

    /// Constructs a default recognizer and asserts each `(name, expected)` case.
    fn assert_recognition(cases: &[(&str, Option<CompilerType>)]) {
        assert_cases(&CompilerRecognizer::new(), cases);
    }

    /// Recognizer with the probe disabled. Use this for tests that exercise
    /// the regex/hint layer in isolation, without depending on whatever
    /// `cc`/`c++` resolve to on the host running the test.
    fn no_probe_recognizer() -> CompilerRecognizer {
        CompilerRecognizer::with_probe(CompilerHints::new(), Box::new(NoProbe))
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_gcc_recognition() {
        // Pure regex behavior. The bare names `cc` and `c++` are
        // intentionally absent from the gcc.yaml regex: they are
        // ambiguous (Linux=GCC, BSDs/macOS=Clang) and dispatch is owned
        // by the probe. Tests for those names live in the probe_* group.
        assert_cases(
            &no_probe_recognizer(),
            &[
                // Basic GCC names
                ("gcc", Some(CompilerType::compiler("gcc"))),
                ("g++", Some(CompilerType::compiler("gcc"))),
                // Cross-compilation variants
                ("arm-linux-gnueabi-gcc", Some(CompilerType::compiler("gcc"))),
                ("aarch64-linux-gnu-g++", Some(CompilerType::compiler("gcc"))),
                ("x86_64-w64-mingw32-gcc", Some(CompilerType::compiler("gcc"))),
                // Versioned variants
                ("gcc-9", Some(CompilerType::compiler("gcc"))),
                ("g++-11", Some(CompilerType::compiler("gcc"))),
                ("gcc-11.2", Some(CompilerType::compiler("gcc"))),
                ("gcc9", Some(CompilerType::compiler("gcc"))),
                ("g++11", Some(CompilerType::compiler("gcc"))),
                // With full paths
                ("/usr/bin/gcc", Some(CompilerType::compiler("gcc"))),
                ("/opt/gcc/bin/g++", Some(CompilerType::compiler("gcc"))),
            ],
        );
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_clang_recognition() {
        assert_recognition(&[
            // Basic Clang names
            ("clang", Some(CompilerType::compiler("clang"))),
            ("clang++", Some(CompilerType::compiler("clang"))),
            // Cross-compilation variants
            ("aarch64-linux-gnu-clang", Some(CompilerType::compiler("clang"))),
            ("arm-linux-gnueabi-clang++", Some(CompilerType::compiler("clang"))),
            // Versioned variants
            ("clang-15", Some(CompilerType::compiler("clang"))),
            ("clang++-16", Some(CompilerType::compiler("clang"))),
            ("clang15", Some(CompilerType::compiler("clang"))),
            ("clang++16", Some(CompilerType::compiler("clang"))),
            ("clang-15.0", Some(CompilerType::compiler("clang"))),
            // With full paths
            ("/usr/bin/clang", Some(CompilerType::compiler("clang"))),
            ("/opt/llvm/bin/clang++", Some(CompilerType::compiler("clang"))),
        ]);
    }

    #[test]
    fn test_windows_exe_extensions() {
        let recognizer = CompilerRecognizer::new();

        // GCC with .exe extensions. (`cc.exe`/`c++.exe` are intentionally
        // absent: those names are ambiguous and the probe owns dispatch
        // for them; the regex returns no match.)
        assert_eq!(recognizer.recognize(path("gcc.exe")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("g++.exe")), Some(CompilerType::compiler("gcc")));

        // Cross-compilation variants with .exe
        assert_eq!(
            recognizer.recognize(path("arm-linux-gnueabi-gcc.exe")),
            Some(CompilerType::compiler("gcc"))
        );
        assert_eq!(
            recognizer.recognize(path("x86_64-w64-mingw32-g++.exe")),
            Some(CompilerType::compiler("gcc"))
        );

        // Versioned variants with .exe
        assert_eq!(recognizer.recognize(path("gcc-9.exe")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("g++-11.2.exe")), Some(CompilerType::compiler("gcc")));

        // Clang with .exe extensions
        assert_eq!(recognizer.recognize(path("clang.exe")), Some(CompilerType::compiler("clang")));
        assert_eq!(recognizer.recognize(path("clang++.exe")), Some(CompilerType::compiler("clang")));
        assert_eq!(recognizer.recognize(path("clang-15.exe")), Some(CompilerType::compiler("clang")));

        // Fortran with .exe extensions
        assert_eq!(recognizer.recognize(path("gfortran.exe")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("flang.exe")), Some(CompilerType::compiler("flang")));
        assert_eq!(recognizer.recognize(path("f95.exe")), Some(CompilerType::compiler("gcc")));

        // Intel Fortran with .exe extensions
        assert_eq!(recognizer.recognize(path("ifort.exe")), Some(CompilerType::compiler("intel_fortran")));
        assert_eq!(recognizer.recognize(path("ifx.exe")), Some(CompilerType::compiler("intel_fortran")));

        // Cray Fortran with .exe extensions
        assert_eq!(recognizer.recognize(path("crayftn.exe")), Some(CompilerType::compiler("cray_fortran")));
        assert_eq!(recognizer.recognize(path("ftn.exe")), Some(CompilerType::compiler("cray_fortran")));

        // CUDA with .exe extensions
        assert_eq!(recognizer.recognize(path("nvcc.exe")), Some(CompilerType::compiler("cuda")));

        // Wrapper tools with .exe extensions
        assert_eq!(recognizer.recognize(path("ccache.exe")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("distcc.exe")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("sccache.exe")), Some(CompilerType::Wrapper));
    }

    #[test]
    fn test_windows_paths_with_exe() {
        let recognizer = CompilerRecognizer::new();

        // Simple Unix-style paths with .exe (should work cross-platform)
        assert_eq!(recognizer.recognize(path("/mingw64/bin/gcc.exe")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("/usr/bin/clang.exe")), Some(CompilerType::compiler("clang")));
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_fortran_recognition() {
        assert_recognition(&[
            // Basic Fortran names
            ("gfortran", Some(CompilerType::compiler("gcc"))),
            ("f95", Some(CompilerType::compiler("gcc"))),
            ("flang", Some(CompilerType::compiler("flang"))),
            ("flang-new", Some(CompilerType::compiler("flang"))),
            // Cross-compilation variants
            ("arm-linux-gnueabi-gfortran", Some(CompilerType::compiler("gcc"))),
            // Versioned variants
            ("gfortran-11", Some(CompilerType::compiler("gcc"))),
            ("gfortran11", Some(CompilerType::compiler("gcc"))),
            ("f95-4.8", Some(CompilerType::compiler("gcc"))),
        ]);
    }

    #[test]
    fn test_intel_fortran_recognition() {
        assert_recognition(&[
            // Intel Fortran names
            ("ifort", Some(CompilerType::compiler("intel_fortran"))),
            ("ifx", Some(CompilerType::compiler("intel_fortran"))),
            // Versioned variants
            ("ifort-2021", Some(CompilerType::compiler("intel_fortran"))),
            ("ifx-2023", Some(CompilerType::compiler("intel_fortran"))),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_cray_fortran_recognition() {
        assert_recognition(&[
            // Cray Fortran names
            ("crayftn", Some(CompilerType::compiler("cray_fortran"))),
            ("ftn", Some(CompilerType::compiler("cray_fortran"))),
        ]);
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
                Some(CompilerType::compiler("gcc")),
                "Failed for path: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_recognize_with_config_hints() {
        let hints = hints_for(&[("custom-gcc-wrapper", Some("gcc")), ("weird-clang-name", Some("clang"))]);

        let recognizer = CompilerRecognizer::new_with_hints(hints);

        // Configured hints take priority
        assert_eq!(recognizer.recognize(path("custom-gcc-wrapper")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("weird-clang-name")), Some(CompilerType::compiler("clang")));

        // Regex detection still works for non-configured compilers
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("unknown-compiler")), None);
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn test_config_hint_matches_bare_spelling_by_filename() {
        // The configured path is absolute (validation requires it to exist
        // on real hosts); the build spells only the name. NoProbe declines
        // everything and `cc` has no regex, so a hit proves the hint.
        let hints = hints_for(&[("/opt/toolchain/cc", Some("clang"))]);

        let sut = CompilerRecognizer::with_probe(hints, Box::new(NoProbe));

        assert_eq!(sut.recognize(path("cc")), Some(CompilerType::compiler("clang")));
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn test_config_hint_does_not_filename_match_path_spellings() {
        // A path spelling names one concrete binary; it must not borrow the
        // classification of a differently-located configured entry.
        let hints = hints_for(&[("/opt/toolchain/cc", Some("clang"))]);

        let sut = CompilerRecognizer::with_probe(hints, Box::new(NoProbe));

        assert_eq!(sut.recognize(path("/usr/bin/cc")), None);
        assert_eq!(sut.recognize(path("./cc")), None);
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn test_conflicting_filename_hints_first_entry_wins() {
        let hints = hints_for(&[("/opt/a/cc", Some("clang")), ("/opt/b/cc", Some("gcc"))]);

        let sut = CompilerRecognizer::with_probe(hints, Box::new(NoProbe));

        // The bare spelling takes the first configured entry (a warning is
        // logged for the disagreeing second); path spellings keep their own.
        assert_eq!(sut.recognize(path("cc")), Some(CompilerType::compiler("clang")));
        assert_eq!(sut.recognize(path("/opt/a/cc")), Some(CompilerType::compiler("clang")));
        assert_eq!(sut.recognize(path("/opt/b/cc")), Some(CompilerType::compiler("gcc")));
    }

    #[test]
    fn test_hint_builder_rejects_an_unknown_spelling() {
        // Both entry points resolve the spelling; only `add` records a hint.
        let mut hints = CompilerHints::new();

        let sut = hints.add(path("/opt/toolchain/cc"), Some("not-a-compiler"));
        assert!(sut.is_err(), "an unknown spelling must not build a hint");
        assert!(sut.unwrap_err().to_string().contains("unknown compiler id"));

        let sut = CompilerHints::check(Some("not-a-compiler"));
        assert!(sut.is_err(), "an unknown spelling must fail even without a hint");
    }

    #[test]
    fn test_is_compiler_type() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("clang")), Some(CompilerType::compiler("clang")));
        assert_ne!(recognizer.recognize(path("gcc")), Some(CompilerType::compiler("clang")));
        assert_ne!(recognizer.recognize(path("clang")), Some(CompilerType::compiler("gcc")));
    }

    #[test]
    fn test_empty_config() {
        // Test that recognizer with empty config works the same as new()
        let recognizer_new = CompilerRecognizer::new();
        let recognizer_empty_config = CompilerRecognizer::new_with_hints(CompilerHints::new());

        assert_eq!(recognizer_new.recognize(path("gcc")), recognizer_empty_config.recognize(path("gcc")));
        assert_eq!(recognizer_new.recognize(path("clang")), recognizer_empty_config.recognize(path("clang")));
        assert_eq!(
            recognizer_new.recognize(path("unknown")),
            recognizer_empty_config.recognize(path("unknown"))
        );
    }

    #[test]
    fn test_gcc_internal_executables_recognition() {
        assert_recognition(&[
            // GCC internal executables are recognized as GCC type
            ("cc1", Some(CompilerType::compiler("gcc"))),
            ("cc1plus", Some(CompilerType::compiler("gcc"))),
            ("cc1obj", Some(CompilerType::compiler("gcc"))),
            ("cc1objplus", Some(CompilerType::compiler("gcc"))),
            ("collect2", Some(CompilerType::compiler("gcc"))),
            ("f951", Some(CompilerType::compiler("gcc"))),
            ("lto1", Some(CompilerType::compiler("gcc"))),
            // With full paths
            ("/usr/libexec/gcc/x86_64-linux-gnu/11/cc1", Some(CompilerType::compiler("gcc"))),
            ("/usr/lib/gcc/x86_64-linux-gnu/11/cc1plus", Some(CompilerType::compiler("gcc"))),
            // Non-GCC internal executables are not matched by this pattern
            ("cc1foo", None),
            ("foo-cc1", None),
        ]);
    }

    #[test]
    fn test_hint_type_resolution_per_entry() {
        let hints = hints_for(&[
            // Entry with an explicit spelling - should use that type
            ("custom-wrapper", Some("clang")),
            // Entry without a spelling but matching a default pattern - should guess Clang
            ("clang++", None),
            // Entry without a spelling and no pattern match - should fall back to GCC
            ("unknown-compiler", None),
            // Another entry without a spelling, matching the Fortran pattern
            ("gfortran", None),
        ]);

        let recognizer = CompilerRecognizer::new_with_hints(hints);

        // Test explicit 'as' field is used
        assert_eq!(recognizer.recognize(path("custom-wrapper")), Some(CompilerType::compiler("clang")));

        // Test pattern matching works when 'as' is None
        assert_eq!(recognizer.recognize(path("clang++")), Some(CompilerType::compiler("clang")));

        // Test fallback to GCC when no pattern matches
        assert_eq!(recognizer.recognize(path("unknown-compiler")), Some(CompilerType::compiler("gcc")));

        // Test a name the builder never saw falls back to the regex pattern
        assert_eq!(recognizer.recognize(path("unhinted-gcc")), Some(CompilerType::compiler("gcc")));

        // Test Fortran pattern matching when 'as' is None
        assert_eq!(recognizer.recognize(path("gfortran")), Some(CompilerType::compiler("gcc")));
    }

    #[test]
    fn test_cuda_recognition() {
        assert_recognition(&[
            ("nvcc", Some(CompilerType::compiler("cuda"))),
            // Versioned variant
            ("nvcc-12.0", Some(CompilerType::compiler("cuda"))),
            // Cross-compilation variant
            ("aarch64-linux-gnu-nvcc", Some(CompilerType::compiler("cuda"))),
            // Non-CUDA executables don't match: "nvcc-fake" has an invalid
            // version suffix, "not-nvcc-at-all" merely contains the substring.
            ("nvcc-fake", None),
            ("not-nvcc-at-all", None),
            ("gcc", Some(CompilerType::compiler("gcc"))),
        ]);
    }

    // Requirements: recognition-compiler-launchers
    #[test]
    fn test_wrapper_recognition() {
        assert_recognition(&[
            ("ccache", Some(CompilerType::Wrapper)),
            ("distcc", Some(CompilerType::Wrapper)),
            ("sccache", Some(CompilerType::Wrapper)),
            ("icecc", Some(CompilerType::Wrapper)),
            // Full paths
            ("/usr/bin/ccache", Some(CompilerType::Wrapper)),
            ("/opt/distcc/bin/distcc", Some(CompilerType::Wrapper)),
            // Non-wrapper executables don't match
            ("ccache-fake", None),
            ("fake-distcc", None),
            ("not-sccache", None),
            // icerun launches arbitrary commands, not compiler invocations.
            ("icerun", None),
        ]);
    }

    #[test]
    fn test_version_capture_functionality() {
        // Test that the DEFAULT_PATTERNS contain regexes that can extract version numbers
        let recognizer = CompilerRecognizer::new();

        // Test basic dash-separated versions
        assert_eq!(recognizer.recognize(path("gcc-11")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("g++-9.3.0")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("clang-15")), Some(CompilerType::compiler("clang")));
        assert_eq!(recognizer.recognize(path("clang-12.1")), Some(CompilerType::compiler("clang")));
        assert_eq!(recognizer.recognize(path("gfortran-12")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("ifort-2023")), Some(CompilerType::compiler("intel_fortran")));
        assert_eq!(recognizer.recognize(path("nvcc-11.8")), Some(CompilerType::compiler("cuda")));

        // Test underscore-separated versions
        assert_eq!(recognizer.recognize(path("gcc_11")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("clang_15.0.7")), Some(CompilerType::compiler("clang")));

        // Test that non-versioned compilers still work
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("clang")), Some(CompilerType::compiler("clang")));
        assert_eq!(recognizer.recognize(path("gfortran")), Some(CompilerType::compiler("gcc")));

        // Test that wrapper executables don't have version patterns (as expected)
        assert_eq!(recognizer.recognize(path("ccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("ccache-1.0")), None); // No version pattern for wrappers

        // Verify behaviorally that GCC patterns include a versioned variant:
        // a numeric-suffixed name resolves to GCC, while a non-numeric suffix
        // (which the version sub-pattern rejects) does not.
        assert_eq!(recognizer.recognize(path("gcc-11")), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(path("gcc-abc")), None);
    }

    #[test]
    fn test_msvc_recognition() {
        assert_recognition(&[
            ("cl", Some(CompilerType::compiler("msvc"))),
            ("cl.exe", Some(CompilerType::compiler("msvc"))),
            // Internal executables should be recognized as MSVC (then ignored by interpreter)
            ("c1", Some(CompilerType::compiler("msvc"))),
            ("c1xx", Some(CompilerType::compiler("msvc"))),
            ("c2", Some(CompilerType::compiler("msvc"))),
        ]);
    }

    #[test]
    fn test_clang_cl_recognition() {
        assert_recognition(&[
            ("clang-cl", Some(CompilerType::compiler("clang_cl"))),
            ("clang-cl.exe", Some(CompilerType::compiler("clang_cl"))),
            ("clang-cl-17", Some(CompilerType::compiler("clang_cl"))),
        ]);
    }

    #[test]
    fn test_intel_cc_recognition() {
        assert_recognition(&[
            ("icx", Some(CompilerType::compiler("intel_cc"))),
            ("icpx", Some(CompilerType::compiler("intel_cc"))),
            ("icc", Some(CompilerType::compiler("intel_cc"))),
            ("icpc", Some(CompilerType::compiler("intel_cc"))),
            ("icx-2024", Some(CompilerType::compiler("intel_cc"))),
        ]);
    }

    #[test]
    fn test_nvidia_hpc_recognition() {
        assert_recognition(&[
            ("nvc", Some(CompilerType::compiler("nvidia_hpc"))),
            ("nvc++", Some(CompilerType::compiler("nvidia_hpc"))),
            ("nvfortran", Some(CompilerType::compiler("nvidia_hpc"))),
            ("pgcc", Some(CompilerType::compiler("nvidia_hpc"))),
            ("pgc++", Some(CompilerType::compiler("nvidia_hpc"))),
            ("pgfortran", Some(CompilerType::compiler("nvidia_hpc"))),
        ]);
    }

    #[test]
    fn test_armclang_recognition() {
        assert_recognition(&[
            ("armclang", Some(CompilerType::compiler("armclang"))),
            ("armclang++", Some(CompilerType::compiler("armclang"))),
            ("armclang-14", Some(CompilerType::compiler("armclang"))),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_cray_cc_recognition() {
        // Case sensitivity note: on Unix the recognition regex is
        // case-sensitive, so "crayCC" only matches because it is listed as
        // its own literal alternative in cray_cc.yaml, not because "craycc"
        // case-folds to it. Both spellings must resolve independently.
        assert_recognition(&[
            ("craycc", Some(CompilerType::compiler("cray_cc"))),
            ("crayCC", Some(CompilerType::compiler("cray_cc"))),
            ("craycxx", Some(CompilerType::compiler("cray_cc"))),
            // Versioned variant
            ("craycc-17", Some(CompilerType::compiler("cray_cc"))),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_amd_compiler_recognition() {
        assert_recognition(&[
            ("amdclang", Some(CompilerType::compiler("clang"))),
            ("amdclang++", Some(CompilerType::compiler("clang"))),
            ("hipcc", Some(CompilerType::compiler("clang"))),
            ("amdflang", Some(CompilerType::compiler("flang"))),
            // A GPU-arch reporting tool, not a compiler driver -- must not be recognized.
            ("amdgpu-arch", None),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_mpi_wrapper_recognition() {
        assert_recognition(&[
            ("mpicc", Some(CompilerType::compiler("mpi"))),
            ("mpicxx", Some(CompilerType::compiler("mpi"))),
            ("mpic++", Some(CompilerType::compiler("mpi"))),
            ("mpiCC", Some(CompilerType::compiler("mpi"))),
            ("mpifort", Some(CompilerType::compiler("mpi"))),
            ("mpif77", Some(CompilerType::compiler("mpi"))),
            ("mpif90", Some(CompilerType::compiler("mpi"))),
            // Versioned variant
            ("mpicc-14", Some(CompilerType::compiler("mpi"))),
            // MPI launchers execute programs, they do not compile -- must not be recognized.
            ("mpirun", None),
            ("mpiexec", None),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_intel_mpi_wrapper_recognition() {
        assert_recognition(&[
            ("mpiicc", Some(CompilerType::compiler("intel_cc"))),
            ("mpiicpc", Some(CompilerType::compiler("intel_cc"))),
            ("mpiicx", Some(CompilerType::compiler("intel_cc"))),
            ("mpiicpx", Some(CompilerType::compiler("intel_cc"))),
            ("mpiifort", Some(CompilerType::compiler("intel_fortran"))),
            ("mpiifx", Some(CompilerType::compiler("intel_fortran"))),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_qnx_recognition() {
        assert_recognition(&[
            ("qcc", Some(CompilerType::compiler("qnx"))),
            ("q++", Some(CompilerType::compiler("qnx"))),
            // A name that merely shares the "q" prefix is not a QNX driver.
            ("qnxcc", None),
            ("qcc-fake", None),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_assembler_recognition() {
        assert_recognition(&[
            ("nasm", Some(CompilerType::compiler("nasm"))),
            ("yasm", Some(CompilerType::compiler("nasm"))),
            ("fasm", Some(CompilerType::compiler("fasm"))),
            // The GNU assembler is deliberately not recognized: gcc/clang
            // spawn it internally on temporary .s files during ordinary
            // compiles (see the recognition-compiler-names Deliberately
            // not recognized table).
            ("as", None),
            ("gas", None),
            // A name that merely shares the "nasm" prefix is not NASM.
            ("nasm-doc", None),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_emscripten_and_ti_recognition() {
        assert_recognition(&[
            // Emscripten drivers, including the .py-suffixed spellings some
            // installs expose. tiarmclang is a single token (no hyphen), so
            // the <prefix>-clang cross rule does not catch it; it must be
            // listed.
            ("emcc", Some(CompilerType::compiler("clang"))),
            ("em++", Some(CompilerType::compiler("clang"))),
            ("emcc.py", Some(CompilerType::compiler("clang"))),
            ("em++.py", Some(CompilerType::compiler("clang"))),
            ("tiarmclang", Some(CompilerType::compiler("clang"))),
            // Emscripten's binutils companions are not compiler drivers.
            ("emar", None),
            ("emranlib", None),
            // The dot in "emcc.py" must match literally, not as a regex
            // wildcard: a name with any other character in that position is
            // not an Emscripten driver.
            ("emccxpy", None),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_swift_recognition() {
        assert_recognition(&[
            ("swiftc", Some(CompilerType::compiler("swift"))),
            // swift-frontend is routed to the Swift type via ignore_when
            // (the same mechanism gcc's cc1 uses) so the interpreter can
            // filter it; the recognizer itself still classifies the
            // basename as Swift.
            ("swift-frontend", Some(CompilerType::compiler("swift"))),
            // `swift` is the package-manager subcommand driver (`swift
            // build`, `swift run`, ...), a different command-line model
            // (subcommand dispatcher, not a compiler invocation) -- it is
            // deliberately not recognized.
            ("swift", None),
        ]);
    }

    // Requirements: recognition-compiler-names
    #[test]
    fn test_microchip_xc8_recognition() {
        assert_recognition(&[
            ("xc8-cc", Some(CompilerType::compiler("gcc"))),
            ("xc8", Some(CompilerType::compiler("gcc"))),
            // The XC8 archiver is not a compiler driver.
            ("xc8-ar", None),
        ]);
    }

    // Coverage lock: these names were already matched by the existing
    // cross-compilation prefix rules (no YAML change); the test pins that
    // behavior so a recognition-pattern refactor cannot silently lose them.
    //
    // Requirements: recognition-compiler-names
    #[test]
    fn test_cross_prefixed_embedded_names_already_recognized() {
        let recognizer = CompilerRecognizer::new();

        for name in ["hexagon-clang", "hexagon-unknown-linux-musl-clang"] {
            assert_eq!(
                recognizer.recognize(path(name)),
                Some(CompilerType::compiler("clang")),
                "name: {}",
                name
            );
        }
        for name in ["xc32-gcc", "riscv64-unknown-elf-gcc"] {
            assert_eq!(
                recognizer.recognize(path(name)),
                Some(CompilerType::compiler("gcc")),
                "name: {}",
                name
            );
        }
    }

    #[test]
    fn test_ibm_xl_recognition() {
        assert_recognition(&[
            ("ibm-clang", Some(CompilerType::compiler("ibm_xl"))),
            ("ibm-clang++", Some(CompilerType::compiler("ibm_xl"))),
            ("xlclang", Some(CompilerType::compiler("ibm_xl"))),
            ("xlclang++", Some(CompilerType::compiler("ibm_xl"))),
        ]);
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
            assert_eq!(upper_gcc, Some(CompilerType::compiler("gcc")));
            assert_eq!(upper_clang, Some(CompilerType::compiler("clang")));
            assert_eq!(mixed_gcc, Some(CompilerType::compiler("gcc")));
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
            assert_eq!(upper_exe, Some(CompilerType::compiler("gcc")));
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
        let probe = Box::new(FakeProbe::new().answer("cc", CompilerType::compiler("clang")));
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::compiler("clang")));
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_classifies_cc_as_gcc_on_linux_like_host() {
        let probe = Box::new(FakeProbe::new().answer("cc", CompilerType::compiler("gcc")));
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::compiler("gcc")));
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
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("cc")), None);
        assert_eq!(recognizer.recognize(path("c++")), None);
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_classifies_cray_prgenv_cc_as_clang_via_cray_banner() {
        // Simulates the HPE Cray PrgEnv wrapper "CC" resolving to the CCE
        // Clang frontend under PrgEnv-cray. The Cray banner ("Cray clang
        // version ...") contains the substring "clang version", which
        // classify_version_output already recognizes -- no change to the
        // classifier itself is needed for this case.
        let probe = Box::new(FakeProbe::new().answer("CC", CompilerType::compiler("clang")));
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("CC")), Some(CompilerType::compiler("clang")));
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_classifies_cray_prgenv_cc_as_gcc_via_fsf_banner() {
        // Simulates "CC" resolving to g++ under PrgEnv-gnu.
        let probe = Box::new(FakeProbe::new().answer("CC", CompilerType::compiler("gcc")));
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("CC")), Some(CompilerType::compiler("gcc")));
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn config_hint_beats_probe_and_suppresses_it_for_cray_prgenv_cc() {
        let hints = hints_for(&[("CC", Some("cray_cc"))]);
        let probe = Box::new(FakeProbe::new().answer("CC", CompilerType::compiler("clang")));
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(hints, probe);

        assert_eq!(recognizer.recognize(path("CC")), Some(CompilerType::compiler("cray_cc")));
        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 0, "hint must short-circuit the probe");
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn probe_inconclusive_for_cray_prgenv_cc_yields_not_recognized() {
        // A programming environment whose compiler prints a banner the
        // probe does not know (e.g. nvc++ under PrgEnv-nvidia) stays
        // unrecognized -- documented limitation in
        // recognition-ambiguous-name-probe.md.
        let probe = Box::new(FakeProbe::new());
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("CC")), None);
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn config_hint_beats_probe_and_suppresses_it() {
        // The user's compilers: entry must win, and the probe must not run
        // when a hint already classifies the path.
        let hints = hints_for(&[("cc", Some("gcc"))]);
        let probe = Box::new(FakeProbe::new().answer("cc", CompilerType::compiler("clang")));
        // Take a raw pointer to the FakeProbe so we can read the call count
        // after handing ownership to the recognizer. Safe because the
        // recognizer owns the box for the duration of the test.
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(hints, probe);

        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::compiler("gcc")));
        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 0, "hint must short-circuit the probe");
    }

    // Requirements: recognition-ambiguous-name-probe
    #[test]
    fn non_ambiguous_names_are_not_probed() {
        let probe = Box::new(FakeProbe::new());
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

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
        // ccache, distcc, sccache, icecc invoked by their own names must
        // reach the regex (which returns CompilerType::Wrapper) so the
        // launcher interpreter unwraps them; a wrapper's own name is not
        // ambiguous and must never enter the probe path.
        let probe = Box::new(FakeProbe::new());
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(path("ccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("distcc")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("sccache")), Some(CompilerType::Wrapper));
        assert_eq!(recognizer.recognize(path("icecc")), Some(CompilerType::Wrapper));

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
                Some(CompilerType::compiler("clang"))
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
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(&link_a), Some(CompilerType::compiler("clang")));
        assert_eq!(recognizer.recognize(&link_b), Some(CompilerType::compiler("clang")));

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "canonicalization in the recognizer must collapse symlinks before the cache lookup"
        );
    }

    // The masquerade contract: an ambiguous basename (`cc`) that
    // canonicalizes to a wrapper binary is probed AS INVOKED -- the
    // masquerade binary answers `--version` in the name it was called
    // by, passing through to the underlying compiler, so probing the
    // invoked path (not the canonical wrapper) yields the real
    // toolchain's banner.
    //
    // Requirements: recognition-ambiguous-name-probe
    #[test]
    #[cfg(unix)]
    fn ambiguous_name_canonicalizing_to_masquerade_wrapper_is_probed_as_invoked() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().expect("tempdir");
        let launcher = dir.path().join("ccache");
        std::fs::write(&launcher, b"").expect("write ccache");

        let farm = dir.path().join("masquerade");
        std::fs::create_dir(&farm).expect("mkdir masquerade");
        let link = farm.join("cc");
        symlink(&launcher, &link).expect("symlink masquerade/cc -> ccache");

        // The canned answer is keyed by the INVOKED path: a hit proves the
        // probe received it rather than the canonical wrapper path.
        let probe = Box::new(FakeProbe::new().answer(link.to_str().unwrap(), CompilerType::compiler("gcc")));
        let probe_ptr: *const FakeProbe = &*probe;
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        let sut = recognizer.recognize(&link);

        assert_eq!(sut, Some(CompilerType::compiler("gcc")));
        let calls = unsafe { (*probe_ptr).calls() };
        assert_eq!(calls, 1, "the masquerade link must be probed exactly once, as invoked");
    }

    // Masquerade links cache under the invoked path, not the canonical
    // one: `cc` and `c++` links to the same wrapper binary answer for
    // different underlying compilers, so collapsing them onto the
    // canonical wrapper key would return the first answer for both.
    //
    // Requirements: recognition-ambiguous-name-probe
    #[test]
    #[cfg(unix)]
    fn masquerade_links_to_same_wrapper_are_probed_separately() {
        use super::super::probe::CachingProbe;
        use std::os::unix::fs::symlink;
        use std::sync::Arc;

        struct CountingProbe {
            calls: Arc<AtomicUsize>,
        }
        impl super::super::probe::CompilerProbe for CountingProbe {
            fn probe(&self, _: &Path) -> Option<CompilerType> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Some(CompilerType::compiler("gcc"))
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let launcher = dir.path().join("ccache");
        std::fs::write(&launcher, b"").expect("write ccache");

        let farm = dir.path().join("masquerade");
        std::fs::create_dir(&farm).expect("mkdir masquerade");
        let link_cc = farm.join("cc");
        let link_cxx = farm.join("c++");
        symlink(&launcher, &link_cc).expect("symlink masquerade/cc -> ccache");
        symlink(&launcher, &link_cxx).expect("symlink masquerade/c++ -> ccache");

        let calls = Arc::new(AtomicUsize::new(0));
        let counting = CountingProbe { calls: Arc::clone(&calls) };
        let probe: Box<dyn super::super::probe::CompilerProbe> = Box::new(CachingProbe::new(counting));
        let recognizer = CompilerRecognizer::with_probe(CompilerHints::new(), probe);

        assert_eq!(recognizer.recognize(&link_cc), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(&link_cxx), Some(CompilerType::compiler("gcc")));
        assert_eq!(recognizer.recognize(&link_cc), Some(CompilerType::compiler("gcc")));

        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "each masquerade name probes once (cached), but cc and c++ must not share an entry"
        );
    }
}

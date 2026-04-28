// SPDX-License-Identifier: GPL-3.0-or-later

//! Unified compiler recognition using regex patterns.
//!
//! This module provides a consolidated approach to recognizing compiler executables
//! using regular expressions instead of separate hard-coded lists and pattern
//! matching functions for each compiler.

use crate::config::{Compiler, CompilerType};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

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
    patterns
        .push((CompilerType::Wrapper, create_compiler_regex(&["ccache", "distcc", "sccache"], false, false)));

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
        other => panic!("Unknown compiler type in YAML: '{}'", other),
    }
}

/// Build a regex that matches any of the given `executables`, with optional
/// cross-compilation prefix and version suffix support, plus `.exe` extension.
fn create_compiler_regex(executables: &[&str], cross_compilation: bool, versioned: bool) -> Regex {
    // Escape for regex (handles '+' in names like "c++", "clang++")
    let escaped: Vec<String> = executables.iter().map(|n| regex::escape(n)).collect();
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

/// A unified compiler recognizer that uses regex patterns
pub struct CompilerRecognizer {
    patterns: Vec<(CompilerType, Regex)>,
    hints: HashMap<PathBuf, CompilerType>,
}

impl CompilerRecognizer {
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

    /// Creates a new compiler recognizer with default patterns
    pub fn new() -> Self {
        Self { patterns: DEFAULT_PATTERNS.clone(), hints: Self::build_hints_map(&[]) }
    }

    /// Creates a new compiler recognizer with configuration-based hints
    ///
    /// # Arguments
    ///
    /// * `compilers` - Slice of compiler configurations with optional type hints
    ///
    /// # Returns
    ///
    /// A new CompilerRecognizer that will prioritize configured hints over regex detection
    pub fn new_with_config(compilers: &[Compiler]) -> Self {
        Self { patterns: DEFAULT_PATTERNS.clone(), hints: Self::build_hints_map(compilers) }
    }

    /// Recognizes the compiler type from an executable path
    ///
    /// This function first checks for configured hints, then falls back to
    /// regex-based detection using the filename.
    ///
    /// # Arguments
    ///
    /// * `executable_path` - The path to the executable (can be relative or absolute)
    ///
    /// # Returns
    ///
    /// `Some(CompilerType)` if the executable is recognized, `None` otherwise
    pub fn recognize(&self, executable_path: &Path) -> Option<CompilerType> {
        // 1. Check configured hints first (by canonical path matching)
        if let Some(hint_type) = self.lookup_hint(executable_path) {
            return Some(hint_type);
        }

        // 2. Check by -v output
        if let Some(verbose_type) = self.recognize_by_verbose_hint(executable_path) {
            return Some(verbose_type);
        }

        // 3. Fall back to regex-based recognition
        self.recognize_by_regex(executable_path)
    }

    /// Looks up a hint for the given executable path
    ///
    /// Tries both the original path and its canonicalized version
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

    /// Internal regex-based recognition
    ///
    /// This function ignores the directory path and only looks at the filename
    /// to determine the compiler type using regex patterns.
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

    /// Internal verbose hint based recognition
    ///
    /// This function ignores all errors in executing binary. Since the binary
    /// can either not exists or not a binary.
    fn recognize_by_verbose_hint(&self, executable_path: &Path) -> Option<CompilerType> {
        let output = Command::new(executable_path).arg("-v").output();
        if output.is_err() {
            return None;
        }

        let output = output.unwrap();
        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stderr);
        if stdout.contains("clang version") {
            Some(CompilerType::Clang)
        } else if stdout.contains("gcc version") {
            Some(CompilerType::Gcc)
        } else {
            None
        }
    }
}

impl Default for CompilerRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn path(s: &str) -> &Path {
        Path::new(s)
    }

    #[test]
    fn test_gcc_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Basic GCC names
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("c++")), Some(CompilerType::Gcc));

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

        // GCC with .exe extensions
        assert_eq!(recognizer.recognize(path("gcc.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("g++.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("cc.exe")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("c++.exe")), Some(CompilerType::Gcc));

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

        // Verify that the patterns created with version capture actually have capture groups
        // by manually testing the regex structure
        let gcc_patterns: Vec<_> = DEFAULT_PATTERNS
            .iter()
            .filter(|(compiler_type, _)| *compiler_type == CompilerType::Gcc)
            .collect();

        // At least one GCC pattern should have capture groups (the versioned one)
        let has_capture_groups = gcc_patterns.iter().any(|(_, regex)| regex.captures_len() > 1);
        assert!(has_capture_groups, "GCC patterns should include version capture groups");
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
}

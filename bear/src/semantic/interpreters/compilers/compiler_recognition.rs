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
use std::sync::LazyLock;

/// Compile-time initialized default regex patterns for compiler recognition
///
/// This method provides built-in patterns for recognizing common compiler executables
/// based on their names. The patterns are designed to handle various scenarios including:
/// - Cross-compilation prefixes (e.g., `arm-linux-gnueabihf-gcc`)
/// - Versioned executables (e.g., `gcc-11`, `clang-15`)
/// - Multiple naming conventions for the same compiler family
///
/// # Supported Compiler Patterns
///
/// - **GCC**: Matches `gcc`, `g++`, `cc`, `c++` with optional cross-compilation prefixes and version suffixes
/// - **Clang**: Matches `clang`, `clang++` with optional cross-compilation prefixes and version suffixes
/// - **Fortran**: Matches `gfortran`, `f77`, `f90`, `f95`, `f03`, `f08` with optional prefixes and versions
/// - **Intel Fortran**: Matches `ifort`, `ifx` with optional version suffixes
/// - **Cray Fortran**: Matches `crayftn`, `ftn` with optional version suffixes
///
/// # Returns
///
/// A vector of tuples where each tuple contains:
/// - `CompilerType`: The type of compiler the pattern identifies
/// - `Regex`: The compiled regular expression pattern for matching executable names
///
/// # Examples
///
/// The returned patterns will match executables like:
/// - `gcc`, `arm-linux-gnueabihf-gcc`, `gcc-11`
/// - `clang++`, `x86_64-pc-linux-gnu-clang`, `clang-15`
/// - `gfortran`, `aarch64-linux-gnu-gfortran-9`
/// - `ifort`, `ifx-2023`
/// - `crayftn`, `ftn`
static DEFAULT_PATTERNS: LazyLock<Vec<(CompilerType, Regex)>> = LazyLock::new(|| {
    /// Helper function for creating compiler regex patterns with platform-specific .exe handling
    ///
    /// # Parameters
    /// - `base_pattern`: The core regex pattern without anchors or .exe suffix
    /// - `with_version_capture`: If true, creates capturing groups for version extraction
    fn create_compiler_regex(base_pattern: &str, with_version: bool) -> Regex {
        let exe_suffix = r"(?:\.exe)?";

        // Add version pattern (with or without capturing group) if requested
        let pattern_with_version = if with_version {
            format!(r"{}(?:[-_]([0-9]+(?:[._-][0-9a-zA-Z]+)*))?", base_pattern)
        } else {
            base_pattern.to_string()
        };

        let full_pattern = format!("^{}{}$", pattern_with_version, exe_suffix);
        Regex::new(&full_pattern)
            .unwrap_or_else(|_| panic!("Invalid regex pattern: {}", full_pattern))
    }
    vec![
        // GCC pattern: matches cc, c++ and gcc cross compilation variants and versioned variants
        (
            CompilerType::Gcc,
            create_compiler_regex(r"(?:[^/]*-)?(?:gcc|g\+\+|cc|c\+\+)", true),
        ),
        // GCC internal executables pattern: matches GCC's internal compiler phases
        // These are implementation details of GCC's compilation process that should be
        // routed to GccInterpreter for proper handling (typically to be ignored).
        // Examples: cc1, cc1plus, cc1obj, cc1objplus, collect2, lto1
        (
            CompilerType::Gcc,
            create_compiler_regex(r"(?:cc1(?:plus|obj|objplus)?|collect2|lto1)", false),
        ),
        // Clang pattern: matches clang, clang++, cross-compilation variants, and versioned variants
        (
            CompilerType::Clang,
            create_compiler_regex(r"(?:[^/]*-)?clang(?:\+\+)?", true),
        ),
        // Fortran pattern: matches gfortran, flang, f77, f90, f95, f03, f08, cross-compilation variants, and versioned variants
        (
            CompilerType::Flang,
            create_compiler_regex(r"(?:[^/]*-)?(?:gfortran|flang|f77|f90|f95|f03|f08)", true),
        ),
        // Intel Fortran pattern: matches ifort, ifx, and versioned variants
        (
            CompilerType::IntelFortran,
            create_compiler_regex(r"(?:ifort|ifx)", true),
        ),
        // Cray Fortran pattern: matches crayftn, ftn
        (
            CompilerType::CrayFortran,
            create_compiler_regex(r"(?:crayftn|ftn)", true),
        ),
        // CUDA pattern: matches nvcc (NVIDIA CUDA Compiler) with optional cross-compilation prefixes and version suffixes
        (
            CompilerType::Cuda,
            create_compiler_regex(r"(?:[^/]*-)?nvcc", true),
        ),
        // Wrapper pattern: matches common compiler wrappers (no version support)
        (
            CompilerType::Wrapper,
            create_compiler_regex(r"(?:ccache|distcc|sccache)", false),
        ),
    ]
});

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
            let canonical_path = compiler
                .path
                .canonicalize()
                .unwrap_or_else(|_| compiler.path.clone());

            let compiler_type = if let Some(as_type) = compiler.as_ {
                // Use explicitly configured compiler type
                as_type
            } else {
                // Guess compiler type using default patterns
                let filename = compiler
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");

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
        Self {
            patterns: DEFAULT_PATTERNS.clone(),
            hints: Self::build_hints_map(&[]),
        }
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
        Self {
            patterns: DEFAULT_PATTERNS.clone(),
            hints: Self::build_hints_map(compilers),
        }
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

        // 2. Fall back to regex-based recognition
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
        if let Ok(canonical_path) = executable_path.canonicalize() {
            if let Some(&compiler_type) = self.hints.get(&canonical_path) {
                return Some(compiler_type);
            }
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
        assert_eq!(recognizer.recognize(path("cc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("c++")), Some(CompilerType::Gcc));

        // Cross-compilation variants
        assert_eq!(
            recognizer.recognize(path("arm-linux-gnueabi-gcc")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("aarch64-linux-gnu-g++")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("x86_64-w64-mingw32-gcc")),
            Some(CompilerType::Gcc)
        );

        // Versioned variants
        assert_eq!(recognizer.recognize(path("gcc-9")), Some(CompilerType::Gcc));
        assert_eq!(
            recognizer.recognize(path("g++-11")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("gcc-11.2")),
            Some(CompilerType::Gcc)
        );

        // With full paths
        assert_eq!(
            recognizer.recognize(path("/usr/bin/gcc")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("/opt/gcc/bin/g++")),
            Some(CompilerType::Gcc)
        );
    }

    #[test]
    fn test_clang_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Basic Clang names
        assert_eq!(
            recognizer.recognize(path("clang")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("clang++")),
            Some(CompilerType::Clang)
        );

        // Cross-compilation variants
        assert_eq!(
            recognizer.recognize(path("aarch64-linux-gnu-clang")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("arm-linux-gnueabi-clang++")),
            Some(CompilerType::Clang)
        );

        // Versioned variants
        assert_eq!(
            recognizer.recognize(path("clang-15")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("clang++-16")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("clang-15.0")),
            Some(CompilerType::Clang)
        );

        // With full paths
        assert_eq!(
            recognizer.recognize(path("/usr/bin/clang")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("/opt/llvm/bin/clang++")),
            Some(CompilerType::Clang)
        );
    }

    #[test]
    fn test_windows_exe_extensions() {
        let recognizer = CompilerRecognizer::new();

        // GCC with .exe extensions
        assert_eq!(
            recognizer.recognize(path("gcc.exe")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("g++.exe")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("cc.exe")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("c++.exe")),
            Some(CompilerType::Gcc)
        );

        // Cross-compilation variants with .exe
        assert_eq!(
            recognizer.recognize(path("arm-linux-gnueabi-gcc.exe")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("x86_64-w64-mingw32-g++.exe")),
            Some(CompilerType::Gcc)
        );

        // Versioned variants with .exe
        assert_eq!(
            recognizer.recognize(path("gcc-9.exe")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("g++-11.2.exe")),
            Some(CompilerType::Gcc)
        );

        // Clang with .exe extensions
        assert_eq!(
            recognizer.recognize(path("clang.exe")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("clang++.exe")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("clang-15.exe")),
            Some(CompilerType::Clang)
        );

        // Fortran with .exe extensions
        assert_eq!(
            recognizer.recognize(path("gfortran.exe")),
            Some(CompilerType::Flang)
        );
        assert_eq!(
            recognizer.recognize(path("flang.exe")),
            Some(CompilerType::Flang)
        );
        assert_eq!(
            recognizer.recognize(path("f77.exe")),
            Some(CompilerType::Flang)
        );
        assert_eq!(
            recognizer.recognize(path("f90.exe")),
            Some(CompilerType::Flang)
        );

        // Intel Fortran with .exe extensions
        assert_eq!(
            recognizer.recognize(path("ifort.exe")),
            Some(CompilerType::IntelFortran)
        );
        assert_eq!(
            recognizer.recognize(path("ifx.exe")),
            Some(CompilerType::IntelFortran)
        );

        // Cray Fortran with .exe extensions
        assert_eq!(
            recognizer.recognize(path("crayftn.exe")),
            Some(CompilerType::CrayFortran)
        );
        assert_eq!(
            recognizer.recognize(path("ftn.exe")),
            Some(CompilerType::CrayFortran)
        );

        // CUDA with .exe extensions
        assert_eq!(
            recognizer.recognize(path("nvcc.exe")),
            Some(CompilerType::Cuda)
        );

        // Wrapper tools with .exe extensions
        assert_eq!(
            recognizer.recognize(path("ccache.exe")),
            Some(CompilerType::Wrapper)
        );
        assert_eq!(
            recognizer.recognize(path("distcc.exe")),
            Some(CompilerType::Wrapper)
        );
        assert_eq!(
            recognizer.recognize(path("sccache.exe")),
            Some(CompilerType::Wrapper)
        );
    }

    #[test]
    fn test_windows_paths_with_exe() {
        let recognizer = CompilerRecognizer::new();

        // Simple Unix-style paths with .exe (should work cross-platform)
        assert_eq!(
            recognizer.recognize(path("/mingw64/bin/gcc.exe")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("/usr/bin/clang.exe")),
            Some(CompilerType::Clang)
        );
    }

    #[test]
    fn test_fortran_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Basic Fortran names
        assert_eq!(
            recognizer.recognize(path("gfortran")),
            Some(CompilerType::Flang)
        );
        assert_eq!(
            recognizer.recognize(path("flang")),
            Some(CompilerType::Flang)
        );
        assert_eq!(recognizer.recognize(path("f77")), Some(CompilerType::Flang));
        assert_eq!(recognizer.recognize(path("f90")), Some(CompilerType::Flang));
        assert_eq!(recognizer.recognize(path("f95")), Some(CompilerType::Flang));
        assert_eq!(recognizer.recognize(path("f03")), Some(CompilerType::Flang));
        assert_eq!(recognizer.recognize(path("f08")), Some(CompilerType::Flang));

        // Cross-compilation variants
        assert_eq!(
            recognizer.recognize(path("arm-linux-gnueabi-gfortran")),
            Some(CompilerType::Flang)
        );

        // Versioned variants
        assert_eq!(
            recognizer.recognize(path("gfortran-11")),
            Some(CompilerType::Flang)
        );
        assert_eq!(
            recognizer.recognize(path("f90-4.8")),
            Some(CompilerType::Flang)
        );
    }

    #[test]
    fn test_intel_fortran_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Intel Fortran names
        assert_eq!(
            recognizer.recognize(path("ifort")),
            Some(CompilerType::IntelFortran)
        );
        assert_eq!(
            recognizer.recognize(path("ifx")),
            Some(CompilerType::IntelFortran)
        );

        // Versioned variants
        assert_eq!(
            recognizer.recognize(path("ifort-2021")),
            Some(CompilerType::IntelFortran)
        );
        assert_eq!(
            recognizer.recognize(path("ifx-2023")),
            Some(CompilerType::IntelFortran)
        );
    }

    #[test]
    fn test_cray_fortran_recognition() {
        let recognizer = CompilerRecognizer::new();

        // Cray Fortran names
        assert_eq!(
            recognizer.recognize(path("crayftn")),
            Some(CompilerType::CrayFortran)
        );
        assert_eq!(
            recognizer.recognize(path("ftn")),
            Some(CompilerType::CrayFortran)
        );
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
        let paths = vec![
            "gcc",
            "./gcc",
            "/usr/bin/gcc",
            "/opt/custom/path/gcc",
            "../../../../some/deep/path/gcc",
        ];

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
        assert_eq!(
            recognizer.recognize(path("custom-gcc-wrapper")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("weird-clang-name")),
            Some(CompilerType::Clang)
        );

        // Regex detection still works for non-configured compilers
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(recognizer.recognize(path("unknown-compiler")), None);
    }

    #[test]
    fn test_is_compiler_type() {
        let recognizer = CompilerRecognizer::new();

        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(
            recognizer.recognize(path("clang")),
            Some(CompilerType::Clang)
        );
        assert_ne!(recognizer.recognize(path("gcc")), Some(CompilerType::Clang));
        assert_ne!(recognizer.recognize(path("clang")), Some(CompilerType::Gcc));
    }

    #[test]
    fn test_empty_config() {
        // Test that recognizer with empty config works the same as new()
        let recognizer_new = CompilerRecognizer::new();
        let recognizer_empty_config = CompilerRecognizer::new_with_config(&[]);

        assert_eq!(
            recognizer_new.recognize(path("gcc")),
            recognizer_empty_config.recognize(path("gcc"))
        );
        assert_eq!(
            recognizer_new.recognize(path("clang")),
            recognizer_empty_config.recognize(path("clang"))
        );
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
        assert_eq!(
            recognizer.recognize(path("cc1plus")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("cc1obj")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("cc1objplus")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("collect2")),
            Some(CompilerType::Gcc)
        );
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
            Compiler {
                path: PathBuf::from("custom-wrapper"),
                as_: Some(CompilerType::Clang),
                ignore: false,
            },
            // Compiler without 'as' field but matches default pattern - should guess Clang
            Compiler {
                path: PathBuf::from("clang++"),
                as_: None,
                ignore: false,
            },
            // Compiler without 'as' field and no pattern match - should fall back to GCC
            Compiler {
                path: PathBuf::from("unknown-compiler"),
                as_: None,
                ignore: false,
            },
            // Ignored compiler - should not be included in hints
            Compiler {
                path: PathBuf::from("ignored-gcc"),
                as_: Some(CompilerType::Gcc),
                ignore: true,
            },
            // Another compiler without 'as' field matching Fortran pattern
            Compiler {
                path: PathBuf::from("gfortran"),
                as_: None,
                ignore: false,
            },
        ];

        let recognizer = CompilerRecognizer::new_with_config(&compilers);

        // Test explicit 'as' field is used
        assert_eq!(
            recognizer.recognize(path("custom-wrapper")),
            Some(CompilerType::Clang)
        );

        // Test pattern matching works when 'as' is None
        assert_eq!(
            recognizer.recognize(path("clang++")),
            Some(CompilerType::Clang)
        );

        // Test fallback to GCC when no pattern matches
        assert_eq!(
            recognizer.recognize(path("unknown-compiler")),
            Some(CompilerType::Gcc)
        );

        // Test ignored compiler is not recognized via hints
        assert_eq!(
            recognizer.recognize(path("ignored-gcc")),
            Some(CompilerType::Gcc) // Should fall back to regex pattern, not hint
        );

        // Test Fortran pattern matching when 'as' is None
        assert_eq!(
            recognizer.recognize(path("gfortran")),
            Some(CompilerType::Flang)
        );
    }

    #[test]
    fn test_cuda_recognition() {
        let recognizer = CompilerRecognizer::default();

        // Test basic CUDA compiler recognition
        assert_eq!(recognizer.recognize(path("nvcc")), Some(CompilerType::Cuda));

        // Test versioned CUDA compiler
        assert_eq!(
            recognizer.recognize(path("nvcc-12.0")),
            Some(CompilerType::Cuda)
        );

        // Test cross-compilation CUDA compiler
        assert_eq!(
            recognizer.recognize(path("aarch64-linux-gnu-nvcc")),
            Some(CompilerType::Cuda)
        );

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
        assert_eq!(
            recognizer.recognize(path("ccache")),
            Some(CompilerType::Wrapper)
        );
        assert_eq!(
            recognizer.recognize(path("distcc")),
            Some(CompilerType::Wrapper)
        );
        assert_eq!(
            recognizer.recognize(path("sccache")),
            Some(CompilerType::Wrapper)
        );

        // Test with full paths
        assert_eq!(
            recognizer.recognize(path("/usr/bin/ccache")),
            Some(CompilerType::Wrapper)
        );
        assert_eq!(
            recognizer.recognize(path("/opt/distcc/bin/distcc")),
            Some(CompilerType::Wrapper)
        );

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
        assert_eq!(
            recognizer.recognize(path("gcc-11")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("g++-9.3.0")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("clang-15")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("clang-12.1")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("gfortran-12")),
            Some(CompilerType::Flang)
        );
        assert_eq!(
            recognizer.recognize(path("ifort-2023")),
            Some(CompilerType::IntelFortran)
        );
        assert_eq!(
            recognizer.recognize(path("nvcc-11.8")),
            Some(CompilerType::Cuda)
        );

        // Test underscore-separated versions
        assert_eq!(
            recognizer.recognize(path("gcc_11")),
            Some(CompilerType::Gcc)
        );
        assert_eq!(
            recognizer.recognize(path("clang_15.0.7")),
            Some(CompilerType::Clang)
        );

        // Test that non-versioned compilers still work
        assert_eq!(recognizer.recognize(path("gcc")), Some(CompilerType::Gcc));
        assert_eq!(
            recognizer.recognize(path("clang")),
            Some(CompilerType::Clang)
        );
        assert_eq!(
            recognizer.recognize(path("gfortran")),
            Some(CompilerType::Flang)
        );

        // Test that wrapper executables don't have version patterns (as expected)
        assert_eq!(
            recognizer.recognize(path("ccache")),
            Some(CompilerType::Wrapper)
        );
        assert_eq!(recognizer.recognize(path("ccache-1.0")), None); // No version pattern for wrappers

        // Verify that the patterns created with version capture actually have capture groups
        // by manually testing the regex structure
        let gcc_patterns: Vec<_> = DEFAULT_PATTERNS
            .iter()
            .filter(|(compiler_type, _)| *compiler_type == CompilerType::Gcc)
            .collect();

        // At least one GCC pattern should have capture groups (the versioned one)
        let has_capture_groups = gcc_patterns
            .iter()
            .any(|(_, regex)| regex.captures_len() > 1);
        assert!(
            has_capture_groups,
            "GCC patterns should include version capture groups"
        );
    }
}

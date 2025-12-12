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
    // GCC pattern: matches cc, c++ and gcc cross compilation variants and versioned variants
    let gcc_pattern = Regex::new(r"^(?:[^/]*-)?(?:gcc|g\+\+|cc|c\+\+)(?:-[\d.]+)?$")
        .expect("Invalid GCC regex pattern");

    // GCC internal executables pattern: matches GCC's internal compiler phases
    // These are implementation details of GCC's compilation process that should be
    // routed to GccInterpreter for proper handling (typically to be ignored).
    // Examples: cc1, cc1plus, cc1obj, cc1objplus, collect2, lto1
    let gcc_internal_pattern = Regex::new(r"^(?:cc1(?:plus|obj|objplus)?|collect2|lto1)$")
        .expect("Invalid GCC internal regex pattern");

    // Clang pattern: matches clang, clang++, cross-compilation variants, and versioned variants
    let clang_pattern = Regex::new(r"^(?:[^/]*-)?clang(?:\+\+)?(?:-[\d.]+)?$")
        .expect("Invalid Clang regex pattern");

    // Fortran pattern: matches gfortran, flang, f77, f90, f95, f03, f08, cross-compilation variants, and versioned variants
    let fortran_pattern =
        Regex::new(r"^(?:[^/]*-)?(?:gfortran|flang|f77|f90|f95|f03|f08)(?:-[\d.]+)?$")
            .expect("Invalid Fortran regex pattern");

    // Intel Fortran pattern: matches ifort, ifx, and versioned variants
    let intel_fortran_pattern =
        Regex::new(r"^(?:ifort|ifx)(?:-[\d.]+)?$").expect("Invalid Intel Fortran regex pattern");

    // Cray Fortran pattern: matches crayftn, ftn
    let cray_fortran_pattern =
        Regex::new(r"^(?:crayftn|ftn)(?:-[\d.]+)?$").expect("Invalid Cray Fortran regex pattern");

    vec![
        (CompilerType::Gcc, gcc_pattern),
        (CompilerType::Gcc, gcc_internal_pattern),
        (CompilerType::Clang, clang_pattern),
        (CompilerType::Flang, fortran_pattern),
        (CompilerType::IntelFortran, intel_fortran_pattern),
        (CompilerType::CrayFortran, cray_fortran_pattern),
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
}

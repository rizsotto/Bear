// SPDX-License-Identifier: GPL-3.0-or-later

//! Unified compiler recognition using regex patterns.
//!
//! This module provides a consolidated approach to recognizing compiler executables
//! using regular expressions instead of separate hard-coded lists and pattern
//! matching functions for each compiler.

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// Compiler types that we can recognize
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerType {
    Gcc,
    Clang,
    Fortran,
    IntelFortran,
    CrayFortran,
}

impl CompilerType {
    /// Returns the display name for this compiler type
    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            CompilerType::Gcc => "GCC",
            CompilerType::Clang => "Clang",
            CompilerType::Fortran => "Fortran",
            CompilerType::IntelFortran => "Intel Fortran",
            CompilerType::CrayFortran => "Cray Fortran",
        }
    }
}

/// A unified compiler recognizer that uses regex patterns
pub struct CompilerRecognizer {
    patterns: HashMap<CompilerType, Regex>,
}

impl CompilerRecognizer {
    /// Creates a new compiler recognizer with default patterns
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // GCC pattern: matches gcc, g++, cc, c++, cross-compilation variants, and versioned variants
        let gcc_pattern = Regex::new(r"^(?:[^/]*-)?(?:gcc|g\+\+|cc|c\+\+)(?:-[\d.]+)?$")
            .expect("Invalid GCC regex pattern");
        patterns.insert(CompilerType::Gcc, gcc_pattern);

        // Clang pattern: matches clang, clang++, cross-compilation variants, and versioned variants
        let clang_pattern = Regex::new(r"^(?:[^/]*-)?clang(?:\+\+)?(?:-[\d.]+)?$")
            .expect("Invalid Clang regex pattern");
        patterns.insert(CompilerType::Clang, clang_pattern);

        // Fortran pattern: matches gfortran, f77, f90, f95, f03, f08, cross-compilation variants, and versioned variants
        let fortran_pattern =
            Regex::new(r"^(?:[^/]*-)?(?:gfortran|f77|f90|f95|f03|f08)(?:-[\d.]+)?$")
                .expect("Invalid Fortran regex pattern");
        patterns.insert(CompilerType::Fortran, fortran_pattern);

        // Intel Fortran pattern: matches ifort, ifx, and versioned variants
        let intel_fortran_pattern = Regex::new(r"^(?:ifort|ifx)(?:-[\d.]+)?$")
            .expect("Invalid Intel Fortran regex pattern");
        patterns.insert(CompilerType::IntelFortran, intel_fortran_pattern);

        // Cray Fortran pattern: matches crayftn, ftn
        let cray_fortran_pattern = Regex::new(r"^(?:crayftn|ftn)(?:-[\d.]+)?$")
            .expect("Invalid Cray Fortran regex pattern");
        patterns.insert(CompilerType::CrayFortran, cray_fortran_pattern);

        Self { patterns }
    }

    /// Recognizes the compiler type from an executable path
    ///
    /// This function ignores the directory path and only looks at the filename
    /// to determine the compiler type.
    ///
    /// # Arguments
    ///
    /// * `executable_path` - The path to the executable (can be relative or absolute)
    ///
    /// # Returns
    ///
    /// `Some(CompilerType)` if the executable is recognized, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use bear::semantic::interpreters::compilers::compiler_recognition::{CompilerRecognizer, CompilerType};
    ///
    /// let recognizer = CompilerRecognizer::new();
    ///
    /// // Basic compiler names
    /// assert_eq!(recognizer.recognize(Path::new("gcc")), Some(CompilerType::Gcc));
    /// assert_eq!(recognizer.recognize(Path::new("clang")), Some(CompilerType::Clang));
    ///
    /// // With full paths (path is ignored)
    /// assert_eq!(recognizer.recognize(Path::new("/usr/bin/gcc")), Some(CompilerType::Gcc));
    /// assert_eq!(recognizer.recognize(Path::new("/opt/clang/bin/clang++")), Some(CompilerType::Clang));
    ///
    /// // Cross-compilation variants
    /// assert_eq!(recognizer.recognize(Path::new("arm-linux-gnueabi-gcc")), Some(CompilerType::Gcc));
    /// assert_eq!(recognizer.recognize(Path::new("aarch64-linux-gnu-clang")), Some(CompilerType::Clang));
    ///
    /// // Versioned variants
    /// assert_eq!(recognizer.recognize(Path::new("gcc-11")), Some(CompilerType::Gcc));
    /// assert_eq!(recognizer.recognize(Path::new("clang-15")), Some(CompilerType::Clang));
    ///
    /// // Unrecognized
    /// assert_eq!(recognizer.recognize(Path::new("unknown-compiler")), None);
    /// ```
    pub fn recognize(&self, executable_path: &Path) -> Option<CompilerType> {
        let filename = executable_path.file_name()?.to_str()?;

        // Check each compiler pattern
        for (&compiler_type, pattern) in &self.patterns {
            if pattern.is_match(filename) {
                return Some(compiler_type);
            }
        }

        None
    }

    /// Checks if an executable is of a specific compiler type
    ///
    /// # Arguments
    ///
    /// * `executable_path` - The path to the executable
    /// * `compiler_type` - The compiler type to check for
    ///
    /// # Returns
    ///
    /// `true` if the executable matches the specified compiler type
    pub fn is_compiler_type(&self, executable_path: &Path, compiler_type: CompilerType) -> bool {
        self.recognize(executable_path) == Some(compiler_type)
    }

    /// Gets all supported compiler types
    #[allow(dead_code)]
    pub fn supported_compilers(&self) -> Vec<CompilerType> {
        self.patterns.keys().copied().collect()
    }

    /// Adds or updates a pattern for a compiler type
    ///
    /// This allows customization of recognition patterns at runtime.
    ///
    /// # Arguments
    ///
    /// * `compiler_type` - The compiler type to add/update
    /// * `pattern` - The regex pattern string
    ///
    /// # Returns
    ///
    /// `Ok(())` if the pattern was valid and added, `Err` with regex error otherwise
    #[allow(dead_code)]
    pub fn add_pattern(
        &mut self,
        compiler_type: CompilerType,
        pattern: &str,
    ) -> Result<(), regex::Error> {
        let regex = Regex::new(pattern)?;
        self.patterns.insert(compiler_type, regex);
        Ok(())
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
            Some(CompilerType::Fortran)
        );
        assert_eq!(
            recognizer.recognize(path("f77")),
            Some(CompilerType::Fortran)
        );
        assert_eq!(
            recognizer.recognize(path("f90")),
            Some(CompilerType::Fortran)
        );
        assert_eq!(
            recognizer.recognize(path("f95")),
            Some(CompilerType::Fortran)
        );
        assert_eq!(
            recognizer.recognize(path("f03")),
            Some(CompilerType::Fortran)
        );
        assert_eq!(
            recognizer.recognize(path("f08")),
            Some(CompilerType::Fortran)
        );

        // Cross-compilation variants
        assert_eq!(
            recognizer.recognize(path("arm-linux-gnueabi-gfortran")),
            Some(CompilerType::Fortran)
        );

        // Versioned variants
        assert_eq!(
            recognizer.recognize(path("gfortran-11")),
            Some(CompilerType::Fortran)
        );
        assert_eq!(
            recognizer.recognize(path("f90-4.8")),
            Some(CompilerType::Fortran)
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
    fn test_is_compiler_type() {
        let recognizer = CompilerRecognizer::new();

        assert!(recognizer.is_compiler_type(path("gcc"), CompilerType::Gcc));
        assert!(recognizer.is_compiler_type(path("clang"), CompilerType::Clang));
        assert!(!recognizer.is_compiler_type(path("gcc"), CompilerType::Clang));
        assert!(!recognizer.is_compiler_type(path("clang"), CompilerType::Gcc));
    }

    #[test]
    fn test_custom_patterns() {
        let mut recognizer = CompilerRecognizer::new();

        // Add a custom pattern for a hypothetical compiler
        assert!(recognizer
            .add_pattern(CompilerType::Gcc, r"^my-custom-gcc$")
            .is_ok());

        // Should now recognize our custom compiler
        assert_eq!(
            recognizer.recognize(path("my-custom-gcc")),
            Some(CompilerType::Gcc)
        );

        // Invalid regex should return error
        assert!(recognizer
            .add_pattern(CompilerType::Gcc, r"[invalid(regex")
            .is_err());
    }

    #[test]
    fn test_supported_compilers() {
        let recognizer = CompilerRecognizer::new();
        let supported = recognizer.supported_compilers();

        // Should have all the compiler types we defined
        assert!(supported.contains(&CompilerType::Gcc));
        assert!(supported.contains(&CompilerType::Clang));
        assert!(supported.contains(&CompilerType::Fortran));
        assert!(supported.contains(&CompilerType::IntelFortran));
        assert!(supported.contains(&CompilerType::CrayFortran));
    }
}

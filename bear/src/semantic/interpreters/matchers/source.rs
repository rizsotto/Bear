// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::path::Path;

#[cfg(target_family = "unix")]
pub fn looks_like_a_source_file(argument: &str) -> bool {
    // not a command line flag
    if argument.starts_with('-') {
        return false;
    }
    if let Some((_, extension)) = argument.rsplit_once('.') {
        return SOURCE_EXTENSIONS.contains(extension);
    }
    false
}

#[cfg(target_family = "windows")]
pub fn looks_like_a_source_file(argument: &str) -> bool {
    // not a command line flag
    if argument.starts_with('/') {
        return false;
    }
    if let Some((_, extension)) = argument.rsplit_once('.') {
        return SOURCE_EXTENSIONS.contains(extension);
    }
    false
}

/// Checks if the given path refers to a binary file (object file or library).
///
/// Binary files are not compilable source files and are typically used as
/// inputs to the linker rather than the compiler.
///
/// # Supported extensions
/// - Object files: `.o`
/// - Static libraries: `.a`, `.lib`
/// - Dynamic libraries: `.so`, `.dylib`, `.dll`
pub fn is_binary_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        BINARY_EXTENSIONS.contains(ext_str.as_str())
    } else {
        false
    }
}

#[rustfmt::skip]
static SOURCE_EXTENSIONS: std::sync::LazyLock<HashSet<&'static str>> = std::sync::LazyLock::new(|| {
    HashSet::from([
        // header files
        "h", "hh", "H", "hp", "hxx", "hpp", "HPP", "h++", "tcc",
        // C
        "c", "C",
        // C++
        "cc", "CC", "c++", "C++", "cxx", "cpp", "cp",
        // CUDA
        "cu",
        // ObjectiveC
        "m", "mi", "mm", "M", "mii",
        // Preprocessed
        "i", "ii",
        // Assembly
        "s", "S", "sx", "asm",
        // Fortran
        "f", "for", "ftn",
        "F", "FOR", "fpp", "FPP", "FTN",
        "f90", "f95", "f03", "f08",
        "F90", "F95", "F03", "F08",
        // go
        "go",
        // brig
        "brig",
        // D
        "d", "di", "dd",
        // Ada
        "ads", "abd",
    ])
});

#[rustfmt::skip]
static BINARY_EXTENSIONS: std::sync::LazyLock<HashSet<&'static str>> = std::sync::LazyLock::new(|| {
    HashSet::from([
        // Object files
        "o",
        // Static libraries
        "a", "lib",
        // Dynamic/shared libraries
        "so", "dylib", "dll",
    ])
});

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_filenames() {
        assert!(looks_like_a_source_file("source.c"));
        assert!(looks_like_a_source_file("source.cpp"));
        assert!(looks_like_a_source_file("source.cxx"));
        assert!(looks_like_a_source_file("source.cc"));

        assert!(looks_like_a_source_file("source.h"));
        assert!(looks_like_a_source_file("source.hpp"));

        assert!(!looks_like_a_source_file("gcc"));
        assert!(!looks_like_a_source_file("clang"));
        assert!(!looks_like_a_source_file("-o"));
        assert!(!looks_like_a_source_file("-Wall"));
        assert!(!looks_like_a_source_file("/o"));
    }

    #[test]
    fn test_is_binary_file_object_files() {
        assert!(is_binary_file(Path::new("main.o")));
        assert!(is_binary_file(Path::new("/path/to/file.o")));
        assert!(is_binary_file(Path::new("build/obj/module.o")));
    }

    #[test]
    fn test_is_binary_file_static_libraries() {
        assert!(is_binary_file(Path::new("libfoo.a")));
        assert!(is_binary_file(Path::new("/usr/lib/libm.a")));
        assert!(is_binary_file(Path::new("foo.lib")));
    }

    #[test]
    fn test_is_binary_file_dynamic_libraries() {
        assert!(is_binary_file(Path::new("libfoo.so")));
        assert!(is_binary_file(Path::new("/usr/lib/libm.dylib")));
        assert!(is_binary_file(Path::new("foo.dll")));
    }

    #[test]
    fn test_is_binary_file_case_insensitive() {
        assert!(is_binary_file(Path::new("file.O")));
        assert!(is_binary_file(Path::new("file.SO")));
        assert!(is_binary_file(Path::new("file.DLL")));
        assert!(is_binary_file(Path::new("file.Dylib")));
    }

    #[test]
    fn test_is_binary_file_source_files_not_binary() {
        assert!(!is_binary_file(Path::new("main.c")));
        assert!(!is_binary_file(Path::new("main.cpp")));
        assert!(!is_binary_file(Path::new("header.h")));
        assert!(!is_binary_file(Path::new("module.rs")));
    }

    #[test]
    fn test_is_binary_file_no_extension() {
        assert!(!is_binary_file(Path::new("executable")));
        assert!(!is_binary_file(Path::new("/usr/bin/gcc")));
    }
}

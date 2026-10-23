// SPDX-License-Identifier: GPL-3.0-or-later

use std::env::JoinPathsError;
use std::path::{Path, PathBuf};

pub const KEY_INTERCEPT_STATE: &str = "BEAR_INTERCEPT";

// man page for `ld.so` (Linux dynamic linker/loader)
pub const KEY_OS_PRELOAD_PATH: &str = "LD_PRELOAD";
// man page for `dyld` (macOS dynamic linker)
pub const KEY_OS_MACOS_PRELOAD_PATH: &str = "DYLD_INSERT_LIBRARIES";
pub const KEY_OS_MACOS_FLAT_NAMESPACE: &str = "DYLD_FORCE_FLAT_NAMESPACE";
// man page for `exec` (Linux system call)
pub const KEY_OS_PATH: &str = "PATH";

// Compiler-specific environment variable names, generated from compilers/*.yaml.
include!(concat!(env!("OUT_DIR"), "/env_keys.rs"));

// These run in every intercepted process, the preload library included, so
// they must not allocate.

// https://www.gnu.org/software/make/manual/html_node/Implicit-Variables.html
// Only the program variables. The `*FLAGS` variables are not kept: compilers
// never read them, make expands them into the command line.
const fn make_program_key(key: &str) -> bool {
    matches!(
        key.as_bytes(),
        b"CC" | b"CXX" | b"CPP" | b"FC" | b"AR" | b"AS" | b"M2C" | b"PC" | b"LEX" | b"YACC" | b"LINT"
    )
}

// https://cmake.org/cmake/help/latest/manual/cmake-env-variables.7.html#environment-variables-for-languages
// https://cmake.org/cmake/help/latest/envvar/ASM_DIALECT.html
const fn cmake_program_key(key: &str) -> bool {
    matches!(
        key.as_bytes(),
        b"CUDACXX"
            | b"CUDAHOSTCXX"
            | b"HIPCXX"
            | b"HIPHOSTCXX"
            | b"ISPC"
            | b"OBJC"
            | b"OBJCXX"
            | b"RC"
            | b"SWIFTC"
            | b"SYCLCXX"
            | b"ASM"
            | b"ASM_NASM"
            | b"ASM_MASM"
            | b"ASM_MARMASM"
            | b"ASM_POASM"
            | b"ASM-ATT"
    )
}

// Windows PATH variable is case sensitive and not always capitalized
const fn path_key(key: &str) -> bool {
    key.eq_ignore_ascii_case(KEY_OS_PATH)
}

pub fn relevant_env(key: &str) -> bool {
    matches!(
        key,
        KEY_INTERCEPT_STATE | KEY_OS_PRELOAD_PATH | KEY_OS_MACOS_PRELOAD_PATH | KEY_OS_MACOS_FLAT_NAMESPACE
    ) || program_env(key)
        || COMPILER_ENV_KEYS.contains(&key)
        || path_key(key)
}

pub const fn program_env(key: &str) -> bool {
    make_program_key(key) || cmake_program_key(key)
}

/// Manipulates a `PATH`-like environment variable by inserting a path at the beginning.
///
/// This function ensures that the specified path appears first in a colon-separated
/// list of paths (like `PATH` or `LD_PRELOAD`). If the path already exists elsewhere
/// in the list, it is removed from its current position and moved to the front.
/// This guarantees that the specified path takes precedence over other paths.
///
/// # Arguments
///
/// * `original` - The original PATH-like environment variable value
/// * `first` - The path to insert at the beginning of the path list
///
/// # Returns
///
/// Returns `Ok(String)` containing the updated path list, or `Err(JoinPathsError)`
/// if path manipulation fails due to invalid characters or platform limitations.
///
/// # Behavior
///
/// - If `original` is empty, returns just the `first` path
/// - If `first` already exists in `original`, it's moved to the front
/// - If `first` doesn't exist, it's prepended to the existing paths
/// - Uses platform-appropriate path separators and handles path encoding
pub fn insert_to_path<P: AsRef<Path>>(original: &str, first: P) -> Result<String, JoinPathsError> {
    let first_path = first.as_ref();

    if original.is_empty() {
        return Ok(first_path.to_string_lossy().to_string());
    }

    let mut paths: Vec<PathBuf> =
        std::env::split_paths(original).filter(|path| path.as_path() != first_path).collect();
    paths.insert(0, first_path.to_owned());
    std::env::join_paths(paths).map(|os_string| os_string.into_string().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Assert that the first entry in a path-like string equals the expected value.
    /// Works for PATH, LD_PRELOAD, or any path-separated environment variable.
    fn assert_first_path_entry(expected: &str, path_like: &str) {
        let path_entries: Vec<String> =
            std::env::split_paths(path_like).map(|p| p.to_string_lossy().to_string()).collect();
        let first_entry = path_entries.first().expect("Path-like string should not be empty");

        assert_eq!(
            first_entry, expected,
            "First path entry should match expected. First entry: {}, expected: {}",
            first_entry, expected
        );
    }

    fn assert_path_entry(expected: &str, path_like: &str) {
        let path_entries: Vec<String> =
            std::env::split_paths(path_like).map(|p| p.to_string_lossy().to_string()).collect();

        assert!(
            path_entries.contains(&expected.to_string()),
            "Path entry should contain expected. Path entries: {:?}, expected: {}",
            path_entries,
            expected
        );
    }

    // Requirements: interception-wrapper-mechanism
    #[test]
    fn program_env_accepts_only_make_and_cmake_program_keys() {
        let cases = [
            ("CC", true),
            ("CXX", true),
            ("LINT", true),
            ("CUDACXX", true),
            ("OBJCXX", true),
            ("SWIFTC", true),
            ("ASM", true),
            ("ASM_NASM", true),
            ("ASM-ATT", true),
            ("CFLAGS", false),
            ("LDFLAGS", false),
            ("CUDAFLAGS", false),
            ("CMAKE_C_COMPILER", false),
            ("CARGO", false),
            ("RUSTC", false),
            ("RUSTC_WRAPPER", false),
            ("RUSTFLAGS", false),
            ("cc", false),
            ("PATH", false),
        ];

        for (key, expected) in cases {
            let sut = program_env(key);

            assert_eq!(sut, expected, "program_env({key:?})");
        }
    }

    // Requirements: interception-wrapper-mechanism
    #[test]
    fn relevant_env_keeps_program_path_and_compiler_keys_only() {
        let cases = [
            ("CC", true),
            ("RC", true),
            ("ASM_MARMASM", true),
            ("BEAR_INTERCEPT", true),
            ("LD_PRELOAD", true),
            ("CPATH", true),
            ("PATH", true),
            ("Path", true),
            ("path", true),
            ("CFLAGS", false),
            ("CXXFLAGS", false),
            ("LDLIBS", false),
            ("CARGO", false),
            ("RUSTC", false),
            ("RUSTC_WRAPPER", false),
            ("RUSTFLAGS", false),
            ("HOME", false),
        ];

        for (key, expected) in cases {
            let sut = relevant_env(key);

            assert_eq!(sut, expected, "relevant_env({key:?})");
        }
    }

    #[test]
    fn test_insert_to_path_empty_original() {
        let original = "";
        let first = PathBuf::from("/usr/local/bin");
        let result = insert_to_path(original, first.clone()).unwrap();
        // For empty path case, we just return the path as a string
        assert_first_path_entry(&first.to_string_lossy(), &result);
    }

    #[test]
    fn test_insert_to_path_prepend_new() {
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original =
            std::env::join_paths([usr_bin.clone(), bin.clone()]).unwrap().to_string_lossy().to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Check that the new path is first
        assert_first_path_entry(&usr_local_bin.to_string_lossy(), &result);
        assert_path_entry(&bin.to_string_lossy(), &result);
        assert_path_entry(&usr_bin.to_string_lossy(), &result);
    }

    #[test]
    fn test_insert_to_path_move_existing_to_front() {
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_bin.clone(), usr_local_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Check that the existing path was moved to front
        assert_first_path_entry(&usr_local_bin.to_string_lossy(), &result);
        assert_path_entry(&bin.to_string_lossy(), &result);
        assert_path_entry(&usr_bin.to_string_lossy(), &result);
    }

    #[test]
    fn test_insert_to_path_already_first() {
        let bin = PathBuf::from("/bin");
        let usr_bin = PathBuf::from("/usr/bin");
        let usr_local_bin = PathBuf::from("/usr/local/bin");

        // Join the original paths using platform-specific separator
        let original = std::env::join_paths([usr_local_bin.clone(), usr_bin.clone(), bin.clone()])
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Apply our function
        let result = insert_to_path(&original, usr_local_bin.clone()).unwrap();

        // Check that the path is still first (no change needed)
        assert_first_path_entry(&usr_local_bin.to_string_lossy(), &result);
        assert_path_entry(&bin.to_string_lossy(), &result);
        assert_path_entry(&usr_bin.to_string_lossy(), &result);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_insert_to_path_windows_mingw_preservation() {
        // Test the exact Windows CI failure scenario - MinGW PATH preservation
        let original = "C:\\mingw64\\bin;C:\\Windows\\System32;C:\\Program Files\\Git\\bin";
        let wrapper_dir = "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\bear-xyz";
        let first = PathBuf::from(wrapper_dir);

        let result = insert_to_path(original, first).unwrap();

        // Wrapper should be first in PATH
        assert_first_path_entry(wrapper_dir, &result);
        assert_path_entry("C:\\mingw64\\bin", &result);
        assert_path_entry("C:\\Windows\\System32", &result);
        assert_path_entry("C:\\Program Files\\Git\\bin", &result);
    }
}

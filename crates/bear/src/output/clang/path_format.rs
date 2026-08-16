// SPDX-License-Identifier: GPL-3.0-or-later

//! This module is responsible for formatting paths in the compiler calls.
//! The reason for this is to ensure that the paths are in a consistent format
//! when it comes to the output.
//!
//! The JSON compilation database
//! [format specification](https://clang.llvm.org/docs/JSONCompilationDatabase.html#format)
//! allows the `directory` attribute to be absolute or relative to the current working
//! directory. The `file`, `output` and `arguments` attributes are either absolute or
//! relative to the `directory` attribute.
//!
//! Four resolution strategies are available, selected per field via the
//! `format.paths` configuration (documented in `man/bear.1.md`):
//!
//! - `as-is`: return the path unchanged (no-op; the default).
//! - `absolute`: join with the base directory when relative, then normalize
//!   via `std::path::absolute()`. Does not require the path to exist on disk.
//! - `relative`: compute the path relative to the base directory. Returns
//!   `PathsCannotBeRelative` when the two paths share no common root (e.g.
//!   files on different Windows drive letters).
//! - `canonical`: resolve symlinks and `.`/`..` via `Path::canonicalize()`,
//!   which requires every path component to exist on disk, then strip the
//!   Windows extended-length prefix (`\\?\`) that clangd rejects (issue #683).
//!
//! The `directory` field is formatted using itself as the base; the `file` and
//! `output` fields are resolved against the already-formatted directory. A
//! `directory` that fails to format drops the whole entry; a `file` or `output`
//! that fails falls back to its original path (see `CommandConverter` in
//! `converter.rs`).
//!
//! The source and output paths that reappear in the `arguments` attribute go
//! through the `file` resolver too, so an entry cannot disagree with itself
//! about a path. Paths embedded inside other flags (`-I../include` and its
//! kind) are left untransformed: rewriting them would require a flag-aware
//! path rewriter for every compiler, which is fragile and out of scope.

use crate::config::PathResolver;
use std::io;
use std::path::{Path, PathBuf, absolute};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(PathBuf, PathBuf),
}

/// Function pointer for resolving a path. Parameters are `(base, path)`.
pub type ResolveFn = fn(&Path, &Path) -> Result<PathBuf, FormatError>;

/// Returns the resolver function for the given strategy.
pub fn resolver_for(strategy: PathResolver) -> ResolveFn {
    match strategy {
        PathResolver::AsIs => resolve_as_is,
        PathResolver::Canonical => resolve_canonical,
        PathResolver::Relative => resolve_relative,
        PathResolver::Absolute => absolute_to,
    }
}

fn resolve_as_is(_base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    Ok(path.to_path_buf())
}

fn resolve_canonical(_base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    Ok(strip_windows_extended_length_prefix(path.canonicalize()?))
}

fn resolve_relative(base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    let absolute = absolute_to(base, path)?;
    relative_to(base, &absolute)
}

/// Strip the Windows extended-length path prefix (`\\?\`) if present.
///
/// On Windows, `Path::canonicalize()` returns paths with this prefix,
/// which tools like clangd do not understand. See GitHub issue #683.
fn strip_windows_extended_length_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") { PathBuf::from(stripped) } else { path }
}

/// Compute the absolute path from the root directory if the path is relative.
fn absolute_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    if path.is_absolute() { Ok(absolute(path)?) } else { Ok(absolute(root.join(path))?) }
}

/// Compute the relative path from the root directory.
fn relative_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    // Ensure both paths are absolute for consistent behavior
    let abs_root = absolute(root)?;
    let abs_path = absolute(path)?;

    let mut root_components = abs_root.components();
    let mut path_components = abs_path.components();

    let mut remaining_root_components = Vec::new();
    let mut remaining_path_components = Vec::new();

    // Find the common prefix
    loop {
        let root_comp = root_components.next();
        let path_comp = path_components.next();
        match (root_comp, path_comp) {
            (Some(root), Some(path)) if root != path => {
                remaining_root_components.push(root);
                remaining_root_components.extend(root_components);
                remaining_path_components.push(path);
                remaining_path_components.extend(path_components);
                break;
            }
            (Some(root), None) => {
                remaining_root_components.push(root);
                remaining_root_components.extend(root_components);
                break;
            }
            (None, Some(path)) => {
                remaining_path_components.push(path);
                remaining_path_components.extend(path_components);
                break;
            }
            (None, None) => break,
            _ => continue,
        }
    }

    // Count remaining components in the root to determine how many `..` are needed
    let mut result = PathBuf::new();
    for _ in remaining_root_components {
        result.push(std::path::Component::ParentDir);
    }

    // Add the remaining components of the path
    for comp in remaining_path_components {
        // if comp is a Prefix or RootDir, signal error
        match comp {
            std::path::Component::Normal(_) | std::path::Component::ParentDir => {
                result.push(comp);
            }
            std::path::Component::CurDir => {
                // Ignore this (should not happen since we are working with absolute paths)
            }
            _ => {
                return Err(FormatError::PathsCannotBeRelative(abs_path, abs_root));
            }
        }
    }

    // When root and path are identical, the component walk leaves `result`
    // empty. An empty path is not a valid JSON compilation database value
    // (it fails entry validation downstream), so emit the POSIX "same
    // directory" form instead.
    if result.as_os_str().is_empty() {
        result.push(std::path::Component::CurDir);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PathResolver;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_relative_to_with_relative_paths() {
        // Test that relative_to works correctly with relative input paths
        let root = Path::new("./some/root");
        let path = Path::new("./some/path/file.txt");

        let result = relative_to(root, path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("../path/file.txt"));
    }

    #[test]
    fn test_path_resolver_as_is() {
        let resolve = resolver_for(PathResolver::AsIs);
        let base = PathBuf::from("/base");
        let path = PathBuf::from("some/path");

        let result = resolve(&base, &path).unwrap();
        assert_eq!(result, path);
    }

    #[test]
    fn test_absolute_to() {
        // The test creates a temporary directory and a file in it.
        // Then it verifies that the absolute path of the file is correct.
        //
        // E.g., `/tmp/tmpdir/file.txt` is the absolute path of the file,
        // if `/tmp/tmpdir` is the root directory and `file.txt` is the file.
        let root_dir = tempdir().unwrap();
        let root_dir_path = root_dir.path().canonicalize().unwrap();

        let file_path = root_dir_path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let file_relative_path = PathBuf::from("file.txt");

        let result = absolute_to(&root_dir_path, &file_relative_path).unwrap();
        assert_eq!(result, file_path);

        let result = absolute_to(&root_dir_path, &file_path).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_relative_to() {
        // The test creates two temporary directories and a file in the first one.
        // Then it verifies that the relative path from the second directory to the file
        // in the first directory is correct.
        //
        // E.g., `../tmpdir/file.txt` is the relative path to the file,
        // if `/tmp/tmpdir2` is the root directory and `/tmp/tmpdir/file.txt` is the file.
        let a_dir = tempdir().unwrap();
        let a_dir_path = a_dir.path().canonicalize().unwrap();
        let a_dir_name = a_dir_path.file_name().unwrap();

        let file_path = a_dir_path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let b_dir = tempdir().unwrap();
        let b_dir_path = b_dir.path().canonicalize().unwrap();

        let result = relative_to(&b_dir_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("..").join(a_dir_name).join("file.txt"));

        let result = relative_to(&a_dir_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    #[test]
    fn test_path_resolver_absolute_with_temp_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let resolve = resolver_for(PathResolver::Absolute);
        let base = temp_path.clone();
        let relative_file = PathBuf::from("test.txt");

        let result = resolve(&base, &relative_file).unwrap();
        assert_eq!(result, file_path);
        assert!(result.is_absolute());
    }

    #[test]
    fn test_path_resolver_relative_with_temp_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let resolve = resolver_for(PathResolver::Relative);
        let result = resolve(&temp_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("test.txt"));
    }

    #[test]
    fn test_relative_to_same_path_returns_curdir() {
        // Regression test for GitHub issue #692: `directory: relative` calls
        // relative_to(working_dir, working_dir); the component walk produces
        // no remaining components and must yield "." rather than an empty
        // path (an empty directory field fails Entry::validate).
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let result = relative_to(&temp_path, &temp_path).unwrap();
        assert_eq!(result, PathBuf::from("."));

        let resolve = resolver_for(PathResolver::Relative);
        let via_resolver = resolve(&temp_path, &temp_path).unwrap();
        assert_eq!(via_resolver, PathBuf::from("."));
    }

    #[test]
    fn test_path_resolver_canonical_with_temp_files() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        // Create a test file
        let file_path = temp_path.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let resolve = resolver_for(PathResolver::Canonical);

        // Test with the full file path since canonicalize requires the file to exist
        let result = resolve(&temp_path, &file_path).unwrap();
        // On Windows, canonicalize() adds \\?\ prefix to temp_path (and thus file_path),
        // but the resolver strips it. Compare against the stripped expected path.
        let expected = strip_windows_extended_length_prefix(file_path);
        assert_eq!(result, expected);
        assert!(result.is_absolute());
    }

    #[test]
    fn test_path_resolver_canonical_no_extended_length_prefix() {
        // Regression test for GitHub issue #683:
        // On Windows, Path::canonicalize() returns extended-length paths
        // with "\\?\" prefix that clangd does not understand.
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().canonicalize().unwrap();

        let file_path = temp_path.join("source.c");
        fs::write(&file_path, "int main() {}").unwrap();

        let resolve = resolver_for(PathResolver::Canonical);

        let result = resolve(&temp_path, &file_path).unwrap();
        let result_str = result.to_string_lossy();
        assert!(
            !result_str.starts_with(r"\\?\"),
            "Canonical path should not have extended-length prefix '\\\\?\\': {}",
            result_str
        );

        let dir_result = resolve(&temp_path, &temp_path).unwrap();
        let dir_str = dir_result.to_string_lossy();
        assert!(
            !dir_str.starts_with(r"\\?\"),
            "Canonical directory should not have extended-length prefix '\\\\?\\': {}",
            dir_str
        );
    }

    #[test]
    fn test_strip_windows_extended_length_prefix() {
        // Unit test for the strip function directly
        let normal = PathBuf::from("/tmp/foo/bar");
        assert_eq!(strip_windows_extended_length_prefix(normal.clone()), normal);

        let with_prefix = PathBuf::from(r"\\?\C:\Users\foo\bar");
        let stripped = strip_windows_extended_length_prefix(with_prefix);
        assert_eq!(stripped, PathBuf::from(r"C:\Users\foo\bar"));

        let unc_path = PathBuf::from(r"\\server\share\file");
        assert_eq!(strip_windows_extended_length_prefix(unc_path.clone()), unc_path);
    }
}

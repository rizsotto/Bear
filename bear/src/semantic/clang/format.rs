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
//! The `arguments` attribute contains the compiler flags, where some flags are using
//! file paths. In the current implementation, the `arguments` attribute is not
//! transformed.

use crate::config::PathResolver;
use std::path::{Path, PathBuf};
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(PathBuf, PathBuf),
}

#[derive(Debug, Error)]
pub enum FormatConfigurationError {
    #[error("Only relative paths for 'file' and 'output' when 'directory' is relative")]
    OnlyRelativePaths,
    #[error("Getting current directory failed: {0}")]
    CurrentWorkingDirectory(#[from] io::Error),
}

impl PathResolver {
    fn resolve(&self, base: &Path, path: &Path) -> Result<PathBuf, FormatError> {
        match self {
            PathResolver::Canonical => {
                let result = path.canonicalize()?;
                Ok(result)
            }
            PathResolver::Relative => {
                let absolute = absolute_to(base, path)?;
                relative_to(base, &absolute)
            }
        }
    }
}

/// Compute the absolute path from the root directory if the path is relative.
fn absolute_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    // TODO: instead of calling `canonicalize` on the path, we should use
    //       `path::absolute` when the filesystem access is not allowed.
    if path.is_absolute() {
        Ok(path.canonicalize()?)
    } else {
        Ok(root.join(path).canonicalize()?)
    }
}

/// Compute the relative path from the root directory.
fn relative_to(root: &Path, path: &Path) -> Result<PathBuf, FormatError> {
    // This is a naive implementation that assumes the root is
    // on the same filesystem/volume as the path.
    let mut root_components = root.components();
    let mut path_components = path.components();

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
                return Err(FormatError::PathsCannotBeRelative(
                    path.to_path_buf(),
                    root.to_path_buf(),
                ));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // TODO: Update tests to work with new Arguments trait system
    // Tests temporarily disabled during refactoring

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
        assert_eq!(
            result,
            PathBuf::from("..").join(a_dir_name).join("file.txt")
        );

        let result = relative_to(&a_dir_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
    }
}

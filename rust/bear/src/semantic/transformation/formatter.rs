// SPDX-License-Identifier: GPL-3.0-or-later

//! https://clang.llvm.org/docs/JSONCompilationDatabase.html#format
//!
//! The format specification allows the `directory` attribute to be absolute or relative
//! to the current working directory. The `file`, `output` and `arguments` attributes
//! are either absolute or relative to the `directory` attribute.
//!
//! The `arguments` attribute contains the compiler flags, where some flags are using
//! file paths. In the current implementation, the `arguments` attribute is not
//! transformed.

use super::*;
use std::env;
use std::path;

#[derive(Default, Debug)]
pub enum PathFormatter {
    DoFormat(config::PathFormat, path::PathBuf),
    #[default]
    SkipFormat,
}
#[derive(Debug, Error)]
pub enum Error {
    // FIXME: Should we report the path that failed?
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(path::PathBuf, path::PathBuf),
}

impl PathFormatter {
    pub fn apply(&self, call: semantic::CompilerCall) -> Result<semantic::CompilerCall, Error> {
        match self {
            PathFormatter::SkipFormat => Ok(call),
            PathFormatter::DoFormat(config, cwd) => call.format(config, cwd),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Only relative paths for 'file' and 'output' when 'directory' is relative.")]
    OnlyRelativePaths,
    #[error("Getting current directory failed: {0}")]
    CurrentWorkingDirectory(#[from] io::Error),
}

impl TryFrom<&config::PathFormat> for PathFormatter {
    type Error = ConfigurationError;

    fn try_from(config: &config::PathFormat) -> Result<Self, Self::Error> {
        use config::PathResolver::Relative;

        // When the directory is relative, the file and output must be relative too.
        if config.directory == Relative && (config.file != Relative || config.output != Relative) {
            return Err(ConfigurationError::OnlyRelativePaths);
        }
        Ok(Self::DoFormat(config.clone(), env::current_dir()?))
    }
}

/// Compute the absolute path from the root directory if the path is relative.
fn absolute_to(root: &path::Path, path: &path::Path) -> Result<path::PathBuf, Error> {
    if path.is_absolute() {
        Ok(path.canonicalize()?)
    } else {
        Ok(root.join(path).canonicalize()?)
    }
}

/// Compute the relative path from the root directory.
fn relative_to(root: &path::Path, path: &path::Path) -> Result<path::PathBuf, Error> {
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
    let mut result = path::PathBuf::new();
    for _ in remaining_root_components {
        result.push(path::Component::ParentDir);
    }

    // Add the remaining components of the path
    for comp in remaining_path_components {
        // if comp is a Prefix or RootDir, signal error
        match comp {
            path::Component::Normal(_) | path::Component::ParentDir => {
                result.push(comp);
            }
            path::Component::CurDir => {
                // Ignore this (should not happen since we are working with absolute paths)
            }
            _ => {
                return Err(Error::PathsCannotBeRelative(
                    path.to_path_buf(),
                    root.to_path_buf(),
                ));
            }
        }
    }

    Ok(result)
}

/// Convenient function to resolve the path based on the configuration.
impl config::PathResolver {
    fn resolve(&self, base: &path::Path, path: &path::Path) -> Result<path::PathBuf, Error> {
        match self {
            config::PathResolver::Canonical => {
                let result = path.canonicalize()?;
                Ok(result)
            }
            config::PathResolver::Relative => {
                absolute_to(base, path).and_then(|p| relative_to(base, &p))
            }
        }
    }
}

impl semantic::CompilerCall {
    pub fn format(self, config: &config::PathFormat, cwd: &path::Path) -> Result<Self, Error> {
        // The working directory is usually an absolute path.
        let working_dir = self.working_dir.canonicalize()?;

        Ok(semantic::CompilerCall {
            compiler: self.compiler,
            working_dir: config.directory.resolve(cwd, &working_dir)?,
            passes: self
                .passes
                .into_iter()
                .map(|pass| pass.format(config, &working_dir))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl semantic::CompilerPass {
    pub fn format(
        self,
        config: &config::PathFormat,
        working_dir: &path::Path,
    ) -> Result<Self, Error> {
        match self {
            semantic::CompilerPass::Compile {
                source,
                output,
                flags,
            } => {
                let source = config.file.resolve(working_dir, &source)?;
                let output: Option<path::PathBuf> = output
                    .map(|candidate| config.output.resolve(working_dir, &candidate))
                    .transpose()?;
                Ok(semantic::CompilerPass::Compile {
                    source,
                    output,
                    flags,
                })
            }
            _ => Ok(self),
        }
    }
}

#[cfg(test)]
mod formatter_tests {
    use super::*;
    use crate::config::{PathFormat, PathResolver};
    use crate::semantic::{CompilerCall, CompilerPass};
    use crate::vec_of_strings;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

        let result = absolute_to(&root_dir_path, &root_dir_path).unwrap();
        assert_eq!(result, root_dir_path);
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

    #[test]
    fn test_path_resolver() {
        let root_dir = tempdir().unwrap();
        let root_dir_path = root_dir.path().canonicalize().unwrap();

        let file_path = root_dir_path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let resolver = PathResolver::Canonical;
        let result = resolver.resolve(&root_dir_path, &file_path).unwrap();
        assert_eq!(result, file_path);

        let resolver = PathResolver::Relative;
        let result = resolver.resolve(&root_dir_path, &file_path).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    #[test]
    fn test_path_formatter_skip_format() {
        let formatter = PathFormatter::SkipFormat;

        let input = CompilerCall {
            compiler: PathBuf::from("gcc"),
            working_dir: PathBuf::from("/project"),
            passes: vec![CompilerPass::Compile {
                source: PathBuf::from("main.c"),
                output: PathBuf::from("main.o").into(),
                flags: vec!["-O2".into()],
            }],
        };

        let result = formatter.apply(input.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_path_formatter_do_format() {
        let source_dir = tempdir().unwrap();
        let source_dir_path = source_dir.path().canonicalize().unwrap();
        let source_dir_name = source_dir_path.file_name().unwrap();
        let source_file_path = source_dir_path.join("main.c");
        fs::write(&source_file_path, "int main() {}").unwrap();

        let build_dir = tempdir().unwrap();
        let build_dir_path = build_dir.path().canonicalize().unwrap();
        let build_dir_name = build_dir_path.file_name().unwrap();
        let output_file_path = build_dir_path.join("main.o");
        fs::write(&output_file_path, "object").unwrap();

        let execution_dir = tempdir().unwrap();
        let execution_dir_path = execution_dir.path().canonicalize().unwrap();

        // The entry contains compiler call with absolute paths.
        let input = CompilerCall {
            compiler: PathBuf::from("gcc"),
            working_dir: build_dir_path.to_path_buf(),
            passes: vec![CompilerPass::Compile {
                source: source_file_path.clone(),
                output: output_file_path.clone().into(),
                flags: vec_of_strings!["-O2"],
            }],
        };

        {
            let sut = PathFormatter::DoFormat(
                PathFormat {
                    directory: PathResolver::Canonical,
                    file: PathResolver::Canonical,
                    output: PathResolver::Canonical,
                },
                execution_dir_path.to_path_buf(),
            );

            let expected = CompilerCall {
                compiler: input.compiler.clone(),
                working_dir: build_dir_path.clone(),
                passes: vec![CompilerPass::Compile {
                    source: source_file_path.clone(),
                    output: output_file_path.clone().into(),
                    flags: vec_of_strings!["-O2"],
                }],
            };

            let result = sut.apply(input.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
        {
            let sut = PathFormatter::DoFormat(
                PathFormat {
                    directory: PathResolver::Canonical,
                    file: PathResolver::Relative,
                    output: PathResolver::Relative,
                },
                execution_dir_path.to_path_buf(),
            );

            let expected = CompilerCall {
                compiler: input.compiler.clone(),
                working_dir: build_dir_path.clone(),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("..").join(source_dir_name).join("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec_of_strings!["-O2"],
                }],
            };

            let result = sut.apply(input.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
        {
            let sut = PathFormatter::DoFormat(
                PathFormat {
                    directory: PathResolver::Relative,
                    file: PathResolver::Relative,
                    output: PathResolver::Relative,
                },
                execution_dir_path.to_path_buf(),
            );

            let expected = CompilerCall {
                compiler: input.compiler.clone(),
                working_dir: PathBuf::from("..").join(build_dir_name),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("..").join(source_dir_name).join("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec_of_strings!["-O2"],
                }],
            };

            let result = sut.apply(input.clone());
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_path_formatter_try_from() {
        // Valid configuration: Canonical paths
        let config = PathFormat {
            directory: PathResolver::Canonical,
            file: PathResolver::Canonical,
            output: PathResolver::Canonical,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PathFormatter::DoFormat(..)));

        // Valid configuration: Relative paths
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Relative,
            output: PathResolver::Relative,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PathFormatter::DoFormat(..)));

        // Invalid configuration: Relative directory with canonical file config
        let config = PathFormat {
            directory: PathResolver::Relative,
            file: PathResolver::Canonical,
            output: PathResolver::Relative,
        };
        let result = PathFormatter::try_from(&config);
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            ConfigurationError::OnlyRelativePaths
        ));
    }
}

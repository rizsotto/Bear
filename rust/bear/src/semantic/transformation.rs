// SPDX-License-Identifier: GPL-3.0-or-later

//! Responsible for transforming the compiler calls.
//!
//! It conditionally removes compiler calls based on compiler names or flags.
//! It can also alter the compiler flags of the compiler calls. The actions
//! are defined in the configuration this module is given.

use crate::{config, semantic};
use std::{io, path};
use thiserror::Error;

/// Responsible to transform the semantic of an executed command.
pub trait Transformation: Send {
    fn apply(&self, _: semantic::CompilerCall) -> Result<semantic::CompilerCall, Error>;
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Path canonicalize failed: {0}")]
    PathCanonicalize(#[from] io::Error),
    #[error("Path {0} can't be relative to {1}")]
    PathsCannotBeRelative(path::PathBuf, path::PathBuf),
    #[error("Configuration instructed to filter out")]
    FilteredOut,
}

/// FilterAndFormat is a transformation that filters and formats the compiler calls.
pub struct FilterAndFormat {
    filter: filter::SemanticFilter,
    formatter: formatter::PathFormatter,
}

impl Transformation for FilterAndFormat {
    fn apply(&self, input: semantic::CompilerCall) -> Result<semantic::CompilerCall, Error> {
        let candidate = self.filter.apply(input)?;
        let formatted = self.formatter.apply(candidate)?;
        Ok(formatted)
    }
}

#[derive(Debug, Error)]
pub enum FilterAndFormatError {
    #[error("Semantic filter configuration error: {0}")]
    SemanticFilter(#[from] filter::SemanticFilterError),
    #[error("Path formatter configuration error: {0}")]
    PathFormatter(#[from] formatter::PathFormatterError),
}

impl TryFrom<&config::Output> for FilterAndFormat {
    type Error = FilterAndFormatError;

    fn try_from(value: &config::Output) -> Result<Self, Self::Error> {
        match value {
            config::Output::Clang {
                compilers,
                format,
                sources,
                ..
            } => {
                let filter = compilers.as_slice().try_into()?;
                let formatter = if sources.only_existing_files {
                    (&format.paths).try_into()?
                } else {
                    log::warn!(
                        "The output formatting configuration is ignored. \
                         Access to the filesystem is disabled in source filters."
                    );
                    formatter::PathFormatter::default()
                };

                Ok(FilterAndFormat { filter, formatter })
            }
            config::Output::Semantic { .. } => {
                // This will do no filtering and no formatting.
                let filter = filter::SemanticFilter::default();
                let formatter = formatter::PathFormatter::default();
                Ok(FilterAndFormat { filter, formatter })
            }
        }
    }
}

mod formatter {
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
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    pub enum PathFormatter {
        DoFormat(config::PathFormat, PathBuf),
        #[default]
        SkipFormat,
    }

    impl Transformation for PathFormatter {
        fn apply(&self, call: semantic::CompilerCall) -> Result<semantic::CompilerCall, Error> {
            match self {
                PathFormatter::SkipFormat => Ok(call),
                PathFormatter::DoFormat(config, cwd) => call.format(config, cwd),
            }
        }
    }

    #[derive(Debug, Error)]
    pub enum PathFormatterError {
        #[error("Only relative paths for 'file' and 'output' when 'directory' is relative.")]
        OnlyRelativePaths,
        #[error("Getting current directory failed: {0}")]
        CurrentWorkingDirectory(#[from] io::Error),
    }

    impl TryFrom<&config::PathFormat> for PathFormatter {
        type Error = PathFormatterError;

        fn try_from(config: &config::PathFormat) -> Result<Self, Self::Error> {
            use config::PathResolver::Relative;

            // When the directory is relative, the file and output must be relative too.
            if config.directory == Relative
                && (config.file != Relative || config.output != Relative)
            {
                return Err(PathFormatterError::OnlyRelativePaths);
            }
            Ok(Self::DoFormat(config.clone(), env::current_dir()?))
        }
    }

    /// Compute the absolute path from the root directory if the path is relative.
    fn absolute_to(root: &Path, path: &Path) -> Result<PathBuf, Error> {
        if path.is_absolute() {
            Ok(path.canonicalize()?)
        } else {
            Ok(root.join(path).canonicalize()?)
        }
    }

    /// Compute the relative path from the root directory.
    fn relative_to(root: &Path, path: &Path) -> Result<PathBuf, Error> {
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
        fn resolve(&self, base: &Path, path: &Path) -> Result<PathBuf, Error> {
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
        pub fn format(self, config: &config::PathFormat, cwd: &Path) -> Result<Self, Error> {
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
            working_dir: &Path,
        ) -> Result<Self, Error> {
            match self {
                semantic::CompilerPass::Compile {
                    source,
                    output,
                    flags,
                } => {
                    let source = config.file.resolve(working_dir, &source)?;
                    let output: Option<PathBuf> = output
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
                PathFormatterError::OnlyRelativePaths
            ));
        }
    }
}

mod filter {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Transformation contains rearranged information from the configuration.
    ///
    /// The configuration is a list of instructions on how to transform the compiler call.
    /// The transformation groups the instructions by the compiler path, so it can be
    /// applied to the compiler call when it matches the path.
    #[derive(Default)]
    pub struct SemanticFilter {
        compilers: HashMap<PathBuf, Vec<config::Compiler>>,
    }

    impl Transformation for SemanticFilter {
        fn apply(&self, input: semantic::CompilerCall) -> Result<semantic::CompilerCall, Error> {
            if let Some(configs) = self.compilers.get(&input.compiler) {
                Self::apply_when_match_compiler(configs.as_slice(), input)
            } else {
                Ok(input)
            }
        }
    }

    impl SemanticFilter {
        /// Apply the transformation to the compiler call.
        ///
        /// Multiple configurations can be applied to the same compiler call.
        /// And depending on the instruction from the configuration, the compiler call
        /// can be ignored, modified, or left unchanged. The conditional ignore will
        /// check if the compiler call matches the flags defined in the configuration.
        fn apply_when_match_compiler(
            configs: &[config::Compiler],
            input: semantic::CompilerCall,
        ) -> Result<semantic::CompilerCall, Error> {
            let mut current_input = Some(input);

            for config in configs {
                current_input = match config {
                    config::Compiler {
                        ignore: config::IgnoreOrConsider::Always,
                        ..
                    } => None,
                    config::Compiler {
                        ignore: config::IgnoreOrConsider::Conditional,
                        arguments,
                        ..
                    } => current_input
                        .filter(|input| !Self::match_condition(arguments, &input.passes)),
                    config::Compiler {
                        ignore: config::IgnoreOrConsider::Never,
                        arguments,
                        ..
                    } => current_input.map(|input| semantic::CompilerCall {
                        compiler: input.compiler.clone(),
                        working_dir: input.working_dir.clone(),
                        passes: Self::apply_argument_changes(arguments, input.passes.as_slice()),
                    }),
                };

                if current_input.is_none() {
                    break;
                }
            }
            current_input.ok_or(Error::FilteredOut)
        }

        /// Check if the compiler call matches the condition defined in the configuration.
        ///
        /// Any compiler pass that matches the flags defined in the configuration will cause
        /// the whole compiler call to be ignored.
        fn match_condition(
            arguments: &config::Arguments,
            passes: &[semantic::CompilerPass],
        ) -> bool {
            let match_flags = arguments.match_.as_slice();
            passes.iter().any(|pass| match pass {
                semantic::CompilerPass::Compile { flags, .. } => {
                    flags.iter().any(|flag| match_flags.contains(flag))
                }
                _ => false,
            })
        }

        /// Apply the changes defined in the configuration to the compiler call.
        ///
        /// The changes can be to remove or add flags to the compiler call.
        /// Only the flags will be changed, but applies to all compiler passes.
        fn apply_argument_changes(
            arguments: &config::Arguments,
            passes: &[semantic::CompilerPass],
        ) -> Vec<semantic::CompilerPass> {
            let arguments_to_remove = arguments.remove.as_slice();
            let arguments_to_add = arguments.add.as_slice();

            let mut new_passes = Vec::with_capacity(passes.len());
            for pass in passes {
                match pass {
                    semantic::CompilerPass::Compile {
                        source,
                        output,
                        flags,
                    } => {
                        let mut new_flags = flags.clone();
                        new_flags.retain(|flag| !arguments_to_remove.contains(flag));
                        new_flags.extend(arguments_to_add.iter().cloned());
                        new_passes.push(semantic::CompilerPass::Compile {
                            source: source.clone(),
                            output: output.clone(),
                            flags: new_flags,
                        });
                    }
                    semantic::CompilerPass::Preprocess => {
                        new_passes.push(semantic::CompilerPass::Preprocess)
                    }
                }
            }
            new_passes
        }
    }

    #[derive(Debug, Error)]
    pub enum SemanticFilterError {
        #[error("'Never' or 'Conditional' can't be used after 'Always' for path {0:?}")]
        AfterAlways(path::PathBuf),
        #[error("'Never' can't be used after 'Conditional' for path {0:?}")]
        AfterConditional(path::PathBuf),
        #[error("'Always' or 'Conditional' can't be used after 'Never' for path {0:?}")]
        AfterNever(path::PathBuf),
        #[error("'Always' can't be used multiple times for path {0:?}")]
        MultipleAlways(path::PathBuf),
        #[error("'Conditional' can't be used multiple times for path {0:?}")]
        MultipleConditional(path::PathBuf),
        #[error("'Never' can't be used multiple times for path {0:?}")]
        MultipleNever(path::PathBuf),
        #[error("'Always' can't be used with arguments for path {0:?}")]
        AlwaysWithArguments(path::PathBuf),
        #[error("'Conditional' can't be used without arguments for path {0:?}")]
        ConditionalWithoutMatch(path::PathBuf),
        #[error("'Never' can't be used with arguments for path {0:?}")]
        NeverWithArguments(path::PathBuf),
    }

    impl TryFrom<&[config::Compiler]> for SemanticFilter {
        type Error = SemanticFilterError;

        /// Validate the configuration of the compiler list.
        ///
        /// The validation is done on the individual compiler configuration.
        /// Duplicate paths are allowed in the list, but the semantic of the
        /// configuration should be still consistent with the usage.
        fn try_from(config: &[config::Compiler]) -> Result<Self, Self::Error> {
            use config::{Arguments, IgnoreOrConsider};

            // Group the compilers by path.
            let mut compilers = HashMap::new();
            for compiler in config {
                compilers
                    .entry(compiler.path.clone())
                    .or_insert_with(Vec::new)
                    .push(compiler.clone());
            }
            // Validate the configuration for each compiler path.
            for (path, compilers) in &compilers {
                let mut has_always = false;
                let mut has_conditional = false;
                let mut has_never = false;

                for compiler in compilers {
                    match compiler.ignore {
                        // problems with the order of the configuration
                        IgnoreOrConsider::Conditional if has_conditional => {
                            return Err(SemanticFilterError::MultipleConditional(path.clone()));
                        }
                        IgnoreOrConsider::Always if has_always => {
                            return Err(SemanticFilterError::MultipleAlways(path.clone()));
                        }
                        IgnoreOrConsider::Never if has_never => {
                            return Err(SemanticFilterError::MultipleNever(path.clone()));
                        }
                        IgnoreOrConsider::Always | IgnoreOrConsider::Never if has_conditional => {
                            return Err(SemanticFilterError::AfterConditional(path.clone()));
                        }
                        IgnoreOrConsider::Always | IgnoreOrConsider::Conditional if has_never => {
                            return Err(SemanticFilterError::AfterNever(path.clone()));
                        }
                        IgnoreOrConsider::Never | IgnoreOrConsider::Conditional if has_always => {
                            return Err(SemanticFilterError::AfterAlways(path.clone()));
                        }
                        // problems with the arguments
                        IgnoreOrConsider::Always if compiler.arguments != Arguments::default() => {
                            return Err(SemanticFilterError::AlwaysWithArguments(path.clone()));
                        }
                        IgnoreOrConsider::Conditional if compiler.arguments.match_.is_empty() => {
                            return Err(SemanticFilterError::ConditionalWithoutMatch(path.clone()));
                        }
                        IgnoreOrConsider::Never if !compiler.arguments.match_.is_empty() => {
                            return Err(SemanticFilterError::NeverWithArguments(path.clone()));
                        }
                        // update the flags, no problems found
                        IgnoreOrConsider::Conditional => {
                            has_conditional = true;
                        }
                        IgnoreOrConsider::Always => {
                            has_always = true;
                        }
                        IgnoreOrConsider::Never => {
                            has_never = true;
                        }
                    }
                }
            }
            Ok(Self { compilers })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::{Arguments, Compiler, IgnoreOrConsider};
        use crate::semantic::transformation::Transformation;
        use crate::semantic::{CompilerCall, CompilerPass};
        use std::path::PathBuf;

        #[test]
        fn test_apply_no_filter() {
            let input = CompilerCall {
                compiler: std::path::PathBuf::from("gcc"),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec!["-O2".into()],
                }],
                working_dir: std::path::PathBuf::from("/project"),
            };
            let expected = input.clone();

            let compilers: Vec<Compiler> = vec![];
            let sut = SemanticFilter::try_from(compilers.as_slice());
            assert!(sut.is_ok());

            let result = sut.unwrap().apply(input);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }

        #[test]
        fn test_apply_filter_match() {
            let input = CompilerCall {
                compiler: std::path::PathBuf::from("cc"),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec!["-O2".into()],
                }],
                working_dir: std::path::PathBuf::from("/project"),
            };

            let compilers = vec![Compiler {
                path: std::path::PathBuf::from("cc"),
                ignore: IgnoreOrConsider::Always,
                arguments: Arguments::default(),
            }];

            let sut = SemanticFilter::try_from(compilers.as_slice());
            assert!(sut.is_ok());

            let result = sut.unwrap().apply(input);
            assert!(result.is_err());
        }

        #[test]
        fn test_apply_conditional_match() {
            let input = CompilerCall {
                compiler: std::path::PathBuf::from("gcc"),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec!["-O2".into(), "-Wall".into()],
                }],
                working_dir: std::path::PathBuf::from("/project"),
            };

            let compilers = vec![Compiler {
                path: std::path::PathBuf::from("gcc"),
                ignore: IgnoreOrConsider::Conditional,
                arguments: Arguments {
                    match_: vec!["-O2".into()],
                    ..Arguments::default()
                },
            }];

            let sut = SemanticFilter::try_from(compilers.as_slice());
            assert!(sut.is_ok());

            let result = sut.unwrap().apply(input);
            assert!(result.is_err());
        }

        #[test]
        fn test_apply_ignore_never_modify_arguments() {
            let input = CompilerCall {
                compiler: std::path::PathBuf::from("gcc"),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec!["-O2".into()],
                }],
                working_dir: std::path::PathBuf::from("/project"),
            };

            let expected = CompilerCall {
                compiler: std::path::PathBuf::from("gcc"),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec!["-Wall".into()],
                }],
                working_dir: std::path::PathBuf::from("/project"),
            };

            let compilers = vec![Compiler {
                path: std::path::PathBuf::from("gcc"),
                ignore: IgnoreOrConsider::Never,
                arguments: Arguments {
                    add: vec!["-Wall".into()],
                    remove: vec!["-O2".into()],
                    ..Arguments::default()
                },
            }];

            let sut = SemanticFilter::try_from(compilers.as_slice());
            assert!(sut.is_ok());

            let result = sut.unwrap().apply(input);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }

        #[test]
        fn test_semantic_filter_try_from_valid_configs() {
            let valid_configs = vec![
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Never,
                    arguments: Arguments::default(),
                }],
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/clang"),
                    ignore: IgnoreOrConsider::Never,
                    arguments: Arguments {
                        add: vec!["-Wall".to_string()],
                        remove: vec!["-O2".to_string()],
                        ..Default::default()
                    },
                }],
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Always,
                    arguments: Arguments::default(),
                }],
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/clang"),
                    ignore: IgnoreOrConsider::Conditional,
                    arguments: Arguments {
                        match_: vec!["-DDEBUG".to_string()],
                        ..Default::default()
                    },
                }],
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/clang"),
                    ignore: IgnoreOrConsider::Conditional,
                    arguments: Arguments {
                        match_: vec!["-DDEBUG".to_string()],
                        add: vec!["-Wall".to_string()],
                        remove: vec!["-O2".to_string()],
                    },
                }],
            ];

            for config in valid_configs {
                let result = SemanticFilter::try_from(config.as_slice());
                assert!(
                    result.is_ok(),
                    "Expected valid configuration to pass: {:?}, {}",
                    config,
                    result.err().unwrap()
                );
            }
        }

        #[test]
        fn test_semantic_filter_try_from_invalid_configs() {
            let invalid_configs = vec![
                // Multiple "Always" for the same path
                vec![
                    Compiler {
                        path: PathBuf::from("/usr/bin/gcc"),
                        ignore: IgnoreOrConsider::Always,
                        arguments: Arguments::default(),
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/gcc"),
                        ignore: IgnoreOrConsider::Always,
                        arguments: Arguments::default(),
                    },
                ],
                // "Always" after "Never"
                vec![
                    Compiler {
                        path: PathBuf::from("/usr/bin/gcc"),
                        ignore: IgnoreOrConsider::Never,
                        arguments: Arguments::default(),
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/gcc"),
                        ignore: IgnoreOrConsider::Always,
                        arguments: Arguments::default(),
                    },
                ],
                // "Never" after "Conditional"
                vec![
                    Compiler {
                        path: PathBuf::from("/usr/bin/gcc"),
                        ignore: IgnoreOrConsider::Conditional,
                        arguments: Arguments {
                            match_: vec!["-O2".to_string()],
                            ..Default::default()
                        },
                    },
                    Compiler {
                        path: PathBuf::from("/usr/bin/gcc"),
                        ignore: IgnoreOrConsider::Never,
                        arguments: Arguments::default(),
                    },
                ],
                // "Always" with arguments
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Always,
                    arguments: Arguments {
                        add: vec!["-Wall".to_string()],
                        ..Default::default()
                    },
                }],
                // "Conditional" without match arguments
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Conditional,
                    arguments: Arguments::default(),
                }],
                // "Never" with match arguments
                vec![Compiler {
                    path: PathBuf::from("/usr/bin/gcc"),
                    ignore: IgnoreOrConsider::Never,
                    arguments: Arguments {
                        match_: vec!["-O2".to_string()],
                        ..Default::default()
                    },
                }],
            ];

            for config in invalid_configs {
                let result = SemanticFilter::try_from(config.as_slice());
                assert!(
                    result.is_err(),
                    "Expected invalid configuration to fail: {:?}",
                    config
                );
            }
        }
    }
}

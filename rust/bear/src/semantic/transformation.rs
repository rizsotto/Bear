// SPDX-License-Identifier: GPL-3.0-or-later

//! Responsible for transforming the compiler calls.
//!
//! It conditionally removes compiler calls based on compiler names or flags.
//! It can also alter the compiler flags of the compiler calls. The actions
//! are defined in the configuration this module is given.

use crate::{config, semantic};

/// FilterAndFormat is a transformation that filters and formats the compiler calls.
pub struct FilterAndFormat {
    filter: filter::SemanticFilter,
    formatter: formatter::PathFormatter,
}

impl semantic::Transformation for FilterAndFormat {
    fn apply(&self, input: semantic::CompilerCall) -> anyhow::Result<semantic::CompilerCall> {
        let candidate = self.filter.apply(input)?;
        let formatted = self.formatter.apply(candidate)?;
        Ok(formatted)
    }
}

impl TryFrom<&config::Output> for FilterAndFormat {
    type Error = anyhow::Error;

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

    use crate::{config, semantic};
    use std::env;
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    pub enum PathFormatter {
        DoFormat(config::PathFormat, PathBuf),
        #[default]
        SkipFormat,
    }

    impl semantic::Transformation for PathFormatter {
        fn apply(&self, call: semantic::CompilerCall) -> anyhow::Result<semantic::CompilerCall> {
            match self {
                PathFormatter::SkipFormat => Ok(call),
                PathFormatter::DoFormat(config, cwd) => call.format(config, cwd),
            }
        }
    }

    impl TryFrom<&config::PathFormat> for PathFormatter {
        type Error = anyhow::Error;

        fn try_from(config: &config::PathFormat) -> Result<Self, Self::Error> {
            Ok(Self::DoFormat(config.clone(), env::current_dir()?))
        }
    }

    /// Compute the absolute path from the root directory if the path is relative.
    fn absolute_to(root: &Path, path: &Path) -> anyhow::Result<PathBuf> {
        if path.is_absolute() {
            Ok(path.canonicalize()?)
        } else {
            Ok(root.join(path).canonicalize()?)
        }
    }

    /// Compute the relative path from the root directory.
    fn relative_to(root: &Path, path: &Path) -> anyhow::Result<PathBuf> {
        // The implementation is naive; it assumes that the path is a child of the root.
        let relative_path = path.strip_prefix(root)?;
        Ok(relative_path.to_path_buf())
    }

    /// Convenient function to resolve the path based on the configuration.
    impl config::PathResolver {
        fn resolve(&self, base: &Path, path: &Path) -> anyhow::Result<PathBuf> {
            match self {
                config::PathResolver::Canonical => path.canonicalize().map_err(anyhow::Error::msg),
                config::PathResolver::Relative => {
                    absolute_to(base, path).and_then(|p| relative_to(base, &p))
                }
            }
        }
    }

    impl semantic::CompilerCall {
        pub fn format(self, config: &config::PathFormat, cwd: &Path) -> anyhow::Result<Self> {
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
        ) -> anyhow::Result<Self> {
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
                    Ok::<semantic::CompilerPass, anyhow::Error>(semantic::CompilerPass::Compile {
                        source,
                        output,
                        flags,
                    })
                }
                _ => Ok(self),
            }
        }
    }
}

mod filter {
    use crate::{config, semantic};
    use anyhow::anyhow;
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

    impl semantic::Transformation for SemanticFilter {
        fn apply(&self, input: semantic::CompilerCall) -> anyhow::Result<semantic::CompilerCall> {
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
        ) -> anyhow::Result<semantic::CompilerCall> {
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
            current_input.ok_or(anyhow!("configuration instructed to filter out"))
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

    impl TryFrom<&[config::Compiler]> for SemanticFilter {
        type Error = anyhow::Error;

        fn try_from(config: &[config::Compiler]) -> Result<Self, Self::Error> {
            let mut compilers = HashMap::new();
            for compiler in config {
                compilers
                    .entry(compiler.path.clone())
                    .or_insert_with(Vec::new)
                    .push(compiler.clone());
            }
            Ok(Self { compilers })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::{Arguments, Compiler, IgnoreOrConsider};
        use crate::semantic::{CompilerCall, CompilerPass, Transformation};
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
            let expected = CompilerCall {
                compiler: std::path::PathBuf::from("gcc"),
                passes: vec![CompilerPass::Compile {
                    source: PathBuf::from("main.c"),
                    output: PathBuf::from("main.o").into(),
                    flags: vec!["-O2".into()],
                }],
                working_dir: std::path::PathBuf::from("/project"),
            };

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
    }
}

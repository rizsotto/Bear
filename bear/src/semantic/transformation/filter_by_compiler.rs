// SPDX-License-Identifier: GPL-3.0-or-later

//! Transformation contains rearranged information from the configuration.
//!
//! The configuration is a list of instructions on how to transform the compiler call.
//! The transformations are grouped by the compiler path, so it can be applied to the
//! compiler call when it matches the path.

use super::*;
use crate::semantic::interpreters::generic::{CompilerCall, CompilerPass};
use std::collections::HashMap;
use std::path;

#[derive(Default, Debug)]
pub struct FilterByCompiler {
    compilers: HashMap<path::PathBuf, Vec<config::Compiler>>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration instructed to filter out")]
    FilteredOut,
}

impl FilterByCompiler {
    pub fn apply(&self, input: CompilerCall) -> Result<CompilerCall, Error> {
        if let Some(configs) = self.compilers.get(&input.compiler) {
            Self::apply_when_match_compiler(configs.as_slice(), input)
        } else {
            Ok(input)
        }
    }

    /// Apply the transformation to the compiler call.
    ///
    /// Multiple configurations can be applied to the same compiler call.
    /// And depending on the instruction from the configuration, the compiler call
    /// can be ignored, modified, or left unchanged. The conditional ignoring will
    /// check if the compiler call matches the flags defined in the configuration.
    fn apply_when_match_compiler(
        configs: &[config::Compiler],
        input: CompilerCall,
    ) -> Result<CompilerCall, Error> {
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
                } => current_input.filter(|input| !Self::match_condition(arguments, &input.passes)),
                config::Compiler {
                    ignore: config::IgnoreOrConsider::Never,
                    arguments,
                    ..
                } => current_input.map(|input| CompilerCall {
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
    fn match_condition(arguments: &config::Arguments, passes: &[CompilerPass]) -> bool {
        let match_flags = arguments.match_.as_slice();
        passes.iter().any(|pass| match pass {
            CompilerPass::Compile { flags, .. } => {
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
        passes: &[CompilerPass],
    ) -> Vec<CompilerPass> {
        let arguments_to_remove = arguments.remove.as_slice();
        let arguments_to_add = arguments.add.as_slice();

        let mut new_passes = Vec::with_capacity(passes.len());
        for pass in passes {
            match pass {
                CompilerPass::Compile {
                    source,
                    output,
                    flags,
                } => {
                    let mut new_flags = flags.clone();
                    new_flags.retain(|flag| !arguments_to_remove.contains(flag));
                    new_flags.extend(arguments_to_add.iter().cloned());
                    new_passes.push(CompilerPass::Compile {
                        source: source.clone(),
                        output: output.clone(),
                        flags: new_flags,
                    });
                }
                CompilerPass::Preprocess => new_passes.push(CompilerPass::Preprocess),
            }
        }
        new_passes
    }
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
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

impl TryFrom<&[config::Compiler]> for FilterByCompiler {
    type Error = ConfigurationError;

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
                        return Err(ConfigurationError::MultipleConditional(path.clone()));
                    }
                    IgnoreOrConsider::Always if has_always => {
                        return Err(ConfigurationError::MultipleAlways(path.clone()));
                    }
                    IgnoreOrConsider::Never if has_never => {
                        return Err(ConfigurationError::MultipleNever(path.clone()));
                    }
                    IgnoreOrConsider::Always | IgnoreOrConsider::Never if has_conditional => {
                        return Err(ConfigurationError::AfterConditional(path.clone()));
                    }
                    IgnoreOrConsider::Always | IgnoreOrConsider::Conditional if has_never => {
                        return Err(ConfigurationError::AfterNever(path.clone()));
                    }
                    IgnoreOrConsider::Never | IgnoreOrConsider::Conditional if has_always => {
                        return Err(ConfigurationError::AfterAlways(path.clone()));
                    }
                    // problems with the arguments
                    IgnoreOrConsider::Always if compiler.arguments != Arguments::default() => {
                        return Err(ConfigurationError::AlwaysWithArguments(path.clone()));
                    }
                    IgnoreOrConsider::Conditional if compiler.arguments.match_.is_empty() => {
                        return Err(ConfigurationError::ConditionalWithoutMatch(path.clone()));
                    }
                    IgnoreOrConsider::Never if !compiler.arguments.match_.is_empty() => {
                        return Err(ConfigurationError::NeverWithArguments(path.clone()));
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
        let sut = FilterByCompiler::try_from(compilers.as_slice());
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

        let sut = FilterByCompiler::try_from(compilers.as_slice());
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

        let sut = FilterByCompiler::try_from(compilers.as_slice());
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

        let sut = FilterByCompiler::try_from(compilers.as_slice());
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
            let result = FilterByCompiler::try_from(config.as_slice());
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
            let result = FilterByCompiler::try_from(config.as_slice());
            assert!(
                result.is_err(),
                "Expected invalid configuration to fail: {:?}",
                config
            );
        }
    }
}

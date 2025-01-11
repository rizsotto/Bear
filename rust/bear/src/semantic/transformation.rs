// SPDX-License-Identifier: GPL-3.0-or-later

//! Responsible for transforming the compiler calls.
//!
//! It conditionally removes compiler calls based on compiler names or flags.
//! It can also alter the compiler flags of the compiler calls. The actions
//! are defined in the configuration this module is given.

use crate::{config, semantic};

use std::collections::HashMap;
use std::path::PathBuf;

/// Transformation contains rearranged information from the configuration.
///
/// The configuration is a list of instruction on how to transform the compiler call.
/// The transformation group the instructions by the compiler path, so it can be
/// applied to the compiler call when it matches the path.
#[derive(Debug, PartialEq)]
pub struct Transformation {
    compilers: HashMap<PathBuf, Vec<config::Compiler>>,
}

impl From<&config::Output> for Transformation {
    fn from(config: &config::Output) -> Self {
        match config {
            config::Output::Clang { compilers, .. } => compilers.as_slice().into(),
            config::Output::Semantic { .. } => Transformation::new(),
        }
    }
}

impl From<&[config::Compiler]> for Transformation {
    fn from(config: &[config::Compiler]) -> Self {
        let mut compilers = HashMap::new();
        for compiler in config {
            compilers
                .entry(compiler.path.clone())
                .or_insert_with(Vec::new)
                .push(compiler.clone());
        }
        Transformation { compilers }
    }
}

impl semantic::Transform for Transformation {
    fn apply(&self, input: semantic::CompilerCall) -> Option<semantic::CompilerCall> {
        if let Some(configs) = self.compilers.get(&input.compiler) {
            Self::apply_when_not_empty(configs.as_slice(), input)
        } else {
            Some(input)
        }
    }
}

impl Transformation {
    fn new() -> Self {
        Transformation {
            compilers: HashMap::new(),
        }
    }

    /// Apply the transformation to the compiler call.
    ///
    /// Multiple configurations can be applied to the same compiler call.
    /// And depending on the instruction from the configuration, the compiler call
    /// can be ignored, modified, or left unchanged. The conditional ignore will
    /// check if the compiler call matches the flags defined in the configuration.
    fn apply_when_not_empty(
        configs: &[config::Compiler],
        input: semantic::CompilerCall,
    ) -> Option<semantic::CompilerCall> {
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
                } => current_input.map(|input| semantic::CompilerCall {
                    compiler: input.compiler.clone(),
                    working_dir: input.working_dir.clone(),
                    passes: Transformation::apply_argument_changes(
                        arguments,
                        input.passes.as_slice(),
                    ),
                }),
            };

            if current_input.is_none() {
                break;
            }
        }
        current_input
    }

    /// Check if the compiler call matches the condition defined in the configuration.
    ///
    /// Any compiler pass that matches the flags defined in the configuration will cause
    /// the whole compiler call to be ignored.
    fn match_condition(arguments: &config::Arguments, passes: &[semantic::CompilerPass]) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Arguments, Compiler, IgnoreOrConsider};
    use crate::semantic::{CompilerCall, CompilerPass, Transform};
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

        let sut = Transformation::from(&config::Output::Semantic {});
        let result = sut.apply(input);

        let expected = CompilerCall {
            compiler: std::path::PathBuf::from("gcc"),
            passes: vec![CompilerPass::Compile {
                source: PathBuf::from("main.c"),
                output: PathBuf::from("main.o").into(),
                flags: vec!["-O2".into()],
            }],
            working_dir: std::path::PathBuf::from("/project"),
        };
        assert_eq!(result, Some(expected));
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

        let sut: Transformation = vec![Compiler {
            path: std::path::PathBuf::from("cc"),
            ignore: IgnoreOrConsider::Always,
            arguments: Arguments::default(),
        }]
        .as_slice()
        .into();
        let result = sut.apply(input);
        assert!(result.is_none());
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

        let sut: Transformation = vec![Compiler {
            path: std::path::PathBuf::from("gcc"),
            ignore: IgnoreOrConsider::Conditional,
            arguments: Arguments {
                match_: vec!["-O2".into()],
                ..Arguments::default()
            },
        }]
        .as_slice()
        .into();
        let result = sut.apply(input);
        assert!(result.is_none());
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

        let sut: Transformation = vec![Compiler {
            path: std::path::PathBuf::from("gcc"),
            ignore: IgnoreOrConsider::Never,
            arguments: Arguments {
                add: vec!["-Wall".into()],
                remove: vec!["-O2".into()],
                ..Arguments::default()
            },
        }]
        .as_slice()
        .into();
        let result = sut.apply(input);

        let expected = CompilerCall {
            compiler: std::path::PathBuf::from("gcc"),
            passes: vec![CompilerPass::Compile {
                source: PathBuf::from("main.c"),
                output: PathBuf::from("main.o").into(),
                flags: vec!["-Wall".into()],
            }],
            working_dir: std::path::PathBuf::from("/project"),
        };
        assert_eq!(result, Some(expected));
    }
}

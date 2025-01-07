// SPDX-License-Identifier: GPL-3.0-or-later

//! Responsible for transforming the compiler calls.
//!
//! It conditionally removes compiler calls based on compiler names or flags.
//! It can also alter the compiler flags of the compiler calls. The actions
//! are defined in the configuration this module is given.

use crate::config;
use crate::semantic;
use crate::semantic::Transform;

pub enum Transformation {
    None,
    Config(Vec<config::Compiler>),
}

impl From<&config::Output> for Transformation {
    fn from(config: &config::Output) -> Self {
        match config {
            config::Output::Clang { compilers, .. } => {
                if compilers.is_empty() {
                    Transformation::None
                } else {
                    let compilers = compilers.clone();
                    Transformation::Config(compilers)
                }
            }
            config::Output::Semantic { .. } => Transformation::None,
        }
    }
}

impl Transform for Transformation {
    fn apply(&self, input: semantic::CompilerCall) -> Option<semantic::CompilerCall> {
        let semantic::CompilerCall {
            compiler,
            passes,
            working_dir,
        } = &input;
        match self.lookup(compiler) {
            Some(config::Compiler {
                ignore: config::IgnoreOrConsider::Always,
                ..
            }) => None,
            Some(config::Compiler {
                ignore: config::IgnoreOrConsider::Conditional,
                arguments,
                ..
            }) => {
                if Self::filter(arguments, passes) {
                    None
                } else {
                    Some(input)
                }
            }
            Some(config::Compiler {
                ignore: config::IgnoreOrConsider::Never,
                arguments,
                ..
            }) => {
                let new_passes = Transformation::execute(arguments, passes);
                Some(semantic::CompilerCall {
                    compiler: compiler.clone(),
                    working_dir: working_dir.clone(),
                    passes: new_passes,
                })
            }
            None => Some(input),
        }
    }
}

impl Transformation {
    // TODO: allow multiple matches for the same compiler
    fn lookup(&self, compiler: &std::path::Path) -> Option<&config::Compiler> {
        match self {
            Transformation::Config(compilers) => compilers.iter().find(|c| c.path == compiler),
            _ => None,
        }
    }

    fn filter(arguments: &config::Arguments, passes: &[semantic::CompilerPass]) -> bool {
        let match_flags = arguments.match_.as_slice();
        passes.iter().any(|pass| match pass {
            semantic::CompilerPass::Compile { flags, .. } => {
                flags.iter().any(|flag| match_flags.contains(flag))
            }
            _ => false,
        })
    }

    fn execute(
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

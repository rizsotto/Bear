// SPDX-License-Identifier: GPL-3.0-or-later

//! Responsible for transforming the compiler calls.
//!
//! It conditionally removes compiler calls based on compiler names or flags.
//! It can also alter the compiler flags of the compiler calls. The actions
//! are defined in the configuration this module is given.

mod filter_by_compiler;
mod filter_by_source_dir;
mod formatter;

use crate::config;
use crate::semantic::interpreters::generic::CompilerCall;
use std::io;
use thiserror::Error;

/// Responsible to transform the semantic of an executed command.
pub trait Transformation: Send {
    fn apply(&self, _: CompilerCall) -> Option<CompilerCall>;
}

/// FilterAndFormat is a transformation that filters and formats the compiler calls.
pub struct FilterAndFormat {
    format_canonical: formatter::PathFormatter,
    filter_by_compiler: filter_by_compiler::FilterByCompiler,
    filter_by_source: filter_by_source_dir::FilterBySourceDir,
    format_by_config: formatter::PathFormatter,
}

impl Transformation for FilterAndFormat {
    fn apply(&self, input: CompilerCall) -> Option<CompilerCall> {
        // FIXME: this is ugly, but could not find a better way to do it.
        //        The methods are returning different errors in `Result`.
        //        While this method returns a `Recognition` enum.
        // match self.format_canonical.apply(input) {
        //     Ok(candidate) => match self.filter_by_compiler.apply(candidate) {
        //         Ok(candidate) => match self.filter_by_source.apply(candidate) {
        //             Ok(candidate) => match self.format_by_config.apply(candidate) {
        //                 Ok(candidate) => semantic::Recognition::Success(candidate),
        //                 Err(error) => semantic::Recognition::Error(error.to_string()),
        //             },
        //             Err(error) => semantic::Recognition::Ignored(error.to_string()),
        //         },
        //         Err(error) => semantic::Recognition::Ignored(error.to_string()),
        //     },
        //     Err(error) => semantic::Recognition::Error(error.to_string()),
        // }
        Some(input)
    }
}

#[derive(Debug, Error)]
pub enum FilterAndFormatError {
    #[error("Path formatter configuration error: {0}")]
    PathFormatter(#[from] formatter::ConfigurationError),
    #[error("Compiler filter configuration error: {0}")]
    FilterByCompiler(#[from] filter_by_compiler::ConfigurationError),
    #[error("Source filter configuration error: {0}")]
    FilterBySourceDir(#[from] filter_by_source_dir::ConfigurationError),
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
                if !sources.only_existing_files {
                    log::warn!("Access to the filesystem is disabled in source filters.");
                }
                let format_canonical = if sources.only_existing_files {
                    let canonical_config = config::PathFormat::default();
                    formatter::PathFormatter::try_from(&canonical_config)?
                } else {
                    formatter::PathFormatter::default()
                };
                let filter_by_compiler = compilers.as_slice().try_into()?;
                let filter_by_source = sources.try_into()?;
                let format_by_config = if sources.only_existing_files {
                    formatter::PathFormatter::try_from(&format.paths)?
                } else {
                    formatter::PathFormatter::default()
                };

                Ok(FilterAndFormat {
                    format_canonical,
                    filter_by_compiler,
                    filter_by_source,
                    format_by_config,
                })
            }
            config::Output::Semantic { .. } => {
                let format_canonical = formatter::PathFormatter::default();
                let filter_by_compiler = filter_by_compiler::FilterByCompiler::default();
                let filter_by_source = filter_by_source_dir::FilterBySourceDir::default();
                let format_by_config = formatter::PathFormatter::default();

                Ok(FilterAndFormat {
                    format_canonical,
                    filter_by_compiler,
                    filter_by_source,
                    format_by_config,
                })
            }
        }
    }
}

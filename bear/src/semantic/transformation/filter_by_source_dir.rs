// SPDX-License-Identifier: GPL-3.0-or-later

//! FilterBySourceDir is a transformation that filters the compiler calls
//! based on the source directory. If the compilation has multiple source
//! files, it will ignore the whole compilation if any of the source files
//! matches the filter.

use super::*;
use crate::config;
use crate::semantic::interpreters::generic::{CompilerCall, CompilerPass};
use std::path;

#[derive(Default, Debug)]
pub struct FilterBySourceDir {
    filters: Vec<config::DirectoryFilter>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Configuration instructed to filter out")]
    FilteredOut,
}

impl FilterBySourceDir {
    // FIXME: This is currently ignore the whole compiler call if any of the
    //        pass matches the filter. This should be changed to ignore only the
    //        pass that matches the filter.
    pub fn apply(&self, input: CompilerCall) -> Result<CompilerCall, Error> {
        // Check if the compiler call matches the source directory filter
        for filter in &self.filters {
            // Check the source for each pass
            let matching = input.passes.iter().any(|pass| {
                if let CompilerPass::Compile { source, .. } = pass {
                    // Check if the source is in the filter directory
                    return source.starts_with(&filter.path);
                }
                false
            });
            // If the source matches the filter, we should ignore or include the call
            if matching {
                return if filter.ignore == config::Ignore::Always {
                    // Ignore the compiler call if the source matches the filter
                    Err(Error::FilteredOut)
                } else {
                    // Include the compiler call if the source matches the filter
                    Ok(input)
                };
            }
        }
        // When no matching filter is found, we should not ignore the call
        Ok(input)
    }
}

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Duplicate directory: {0}")]
    DuplicateItem(path::PathBuf),
    #[error("Same directory to include and exclude: {0}")]
    DuplicateInstruction(path::PathBuf),
    // FIXME: Should we report the path that failed?
    #[error("Canonicalization failed: {0}")]
    Canonicalization(#[from] io::Error),
}

impl TryFrom<&config::SourceFilter> for FilterBySourceDir {
    type Error = ConfigurationError;

    // FIXME: Should we check if the allowed directory and the ignored directory are
    //        parents of each other? It make sens if the allowed directory is first,
    //        and the ignored directory is second and parent of the first. But if the
    //        order is reversed, the allowed will never be used.
    fn try_from(value: &config::SourceFilter) -> Result<Self, Self::Error> {
        // Convert the source filter to a list of directory filters
        let filters: Vec<config::DirectoryFilter> = value.try_into()?;
        let mut verified: Vec<config::DirectoryFilter> = vec![];

        // Check the semantics of the filters
        for filter in filters.iter() {
            // Check if the same path is already in the list
            if let Some(duplicate) = verified.iter().find(|f| f.path == filter.path) {
                // Classify the error based on the ignore flag match
                let path = filter.path.clone();
                return if duplicate.ignore == filter.ignore {
                    Err(ConfigurationError::DuplicateItem(path))
                } else {
                    Err(ConfigurationError::DuplicateInstruction(path))
                };
            }
            verified.push(filter.clone());
        }

        Ok(Self { filters })
    }
}

/// Convert the source filter to a list of directory filters.
///
/// The conversion is done by canonicalizing the paths when the filesystem
/// is accessible. Otherwise, the filter paths left as is.
impl TryFrom<&config::SourceFilter> for Vec<config::DirectoryFilter> {
    type Error = io::Error;

    fn try_from(value: &config::SourceFilter) -> Result<Self, Self::Error> {
        let filters = value
            .paths
            .iter()
            .flat_map(|filter| {
                if value.only_existing_files {
                    filter.path.canonicalize().map(|p| config::DirectoryFilter {
                        path: p,
                        ignore: filter.ignore.clone(),
                    })
                } else {
                    Ok(filter.clone())
                }
            })
            .collect();
        Ok(filters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{ConfigurationError, Error, FilterBySourceDir};
    use crate::config::{DirectoryFilter, Ignore, SourceFilter};
    use std::path::PathBuf;

    #[test]
    fn test_filter_by_source_dir_try_from_without_filesystem() {
        let config = SourceFilter {
            only_existing_files: false,
            paths: vec![
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Never,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/tests"),
                    ignore: Ignore::Always,
                },
            ],
        };

        let result: Result<FilterBySourceDir, ConfigurationError> = (&config).try_into();
        assert!(result.is_ok());

        let filter_by_source_dir = result.unwrap();
        assert_eq!(filter_by_source_dir.filters.len(), 2);
        assert_eq!(
            filter_by_source_dir.filters[0].path,
            PathBuf::from("/project/src")
        );
        assert_eq!(filter_by_source_dir.filters[0].ignore, Ignore::Never);
        assert_eq!(
            filter_by_source_dir.filters[1].path,
            PathBuf::from("/project/tests")
        );
        assert_eq!(filter_by_source_dir.filters[1].ignore, Ignore::Always);
    }

    #[test]
    fn test_filter_by_source_dir_duplicate_instruction() {
        let config = SourceFilter {
            only_existing_files: false,
            paths: vec![
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Always,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/test"),
                    ignore: Ignore::Always,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Never,
                },
            ],
        };

        let result: Result<FilterBySourceDir, ConfigurationError> = (&config).try_into();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::DuplicateInstruction(path) if path == PathBuf::from("/project/src")
        ));
    }

    #[test]
    fn test_filter_by_source_dir_duplicate_entry() {
        let config = SourceFilter {
            only_existing_files: false,
            paths: vec![
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Always,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/test"),
                    ignore: Ignore::Never,
                },
                DirectoryFilter {
                    path: PathBuf::from("/project/src"),
                    ignore: Ignore::Always,
                },
            ],
        };

        let result: Result<FilterBySourceDir, ConfigurationError> = (&config).try_into();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigurationError::DuplicateItem(path) if path == PathBuf::from("/project/src")
        ));
    }

    #[test]
    fn test_filter_by_source_dir_apply_filtered_out() {
        let filter = FilterBySourceDir {
            filters: vec![DirectoryFilter {
                path: PathBuf::from("/project/src"),
                ignore: Ignore::Always,
            }],
        };

        let result = filter.apply(COMPILER_CALL.clone());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::FilteredOut));
    }

    #[test]
    fn test_filter_by_source_dir_apply_not_filtered_out_include() {
        let filter = FilterBySourceDir {
            filters: vec![DirectoryFilter {
                path: PathBuf::from("/project/src"),
                ignore: Ignore::Never,
            }],
        };

        let result = filter.apply(COMPILER_CALL.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), COMPILER_CALL.clone());
    }

    #[test]
    fn test_filter_by_source_dir_apply_no_instructions() {
        let filter = FilterBySourceDir { filters: vec![] };

        let result = filter.apply(COMPILER_CALL.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), COMPILER_CALL.clone());
    }

    static COMPILER_CALL: std::sync::LazyLock<CompilerCall> =
        std::sync::LazyLock::new(|| CompilerCall {
            compiler: PathBuf::from("gcc"),
            working_dir: PathBuf::from("/project"),
            passes: vec![CompilerPass::Compile {
                source: PathBuf::from("/project/src/main.c"),
                output: None,
                flags: vec![],
            }],
        });
}

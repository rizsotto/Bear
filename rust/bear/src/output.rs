/*  Copyright (C) 2012-2024 by László Nagy
    This file is part of Bear.

    Bear is a tool to generate compilation database for clang tooling.

    Bear is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Bear is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::{args, config, filter};
use anyhow::{anyhow, Context, Result};
use json_compilation_db::Entry;
use path_absolutize::Absolutize;
use semantic::{CompilerPass, Meaning};
use serde_json::Error;

/// Responsible for writing the final compilation database file.
///
/// Implements filtering, formatting and atomic file writing.
/// (Atomic file writing implemented by writing to a temporary file and renaming it.)
///
/// Filtering is implemented by the `filter` module, and the formatting is implemented by the
/// `json_compilation_db` module.
pub struct OutputWriter {
    output: PathBuf,
    append: bool,
    filter: config::Filter,
    format: config::Format,
}

impl OutputWriter {
    /// Create a new instance of the output writer.
    pub fn configure(args: &args::BuildSemantic, config: &config::Output) -> Self {
        match config {
            config::Output::Clang { format, filter, .. } => OutputWriter {
                output: PathBuf::from(&args.file_name),
                append: args.append,
                filter: validate_filter(filter),
                format: format.clone(),
            },
            config::Output::Semantic { .. } => {
                todo!("implement this case")
            }
        }
    }

    /// Implements the main logic of the output writer.
    pub fn run(&self, meanings: impl Iterator<Item = semantic::Meaning>) -> anyhow::Result<()> {
        let entries = meanings.flat_map(|value| {
            into_entries(value).unwrap_or_else(|error| {
                log::error!(
                    "Failed to convert semantic meaning to compilation database entries: {}",
                    error
                );
                vec![]
            })
        });
        if self.append && self.output.exists() {
            let from_db = Self::read_from_compilation_db(Path::new(&self.output))?;
            let final_entries = entries.chain(from_db);
            self.write_into_compilation_db(final_entries)
        } else {
            if self.append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            self.write_into_compilation_db(entries)
        }
    }

    fn write_into_compilation_db(
        &self,
        entries: impl Iterator<Item = Entry>,
    ) -> anyhow::Result<()> {
        // Filter out the entries as per the configuration.
        let filter: filter::EntryPredicate = TryFrom::try_from(&self.filter)?;
        let filtered_entries = entries.filter(filter);
        // Write the entries to a temporary file.
        self.write_into_temporary_compilation_db(filtered_entries)
            .and_then(|temp| {
                // Rename the temporary file to the final output.
                std::fs::rename(temp.as_path(), self.output.as_path()).with_context(|| {
                    format!(
                        "Failed to rename file from '{:?}' to '{:?}'.",
                        temp.as_path(),
                        self.output.as_path()
                    )
                })
            })
    }

    /// Write the entries to a temporary file and returns the temporary file name.
    fn write_into_temporary_compilation_db(
        &self,
        entries: impl Iterator<Item = Entry>,
    ) -> anyhow::Result<PathBuf> {
        // FIXME: Implement entry formatting.

        // Generate a temporary file name.
        let file_name = self.output.with_extension("tmp");
        // Open the file for writing.
        let file = File::create(&file_name)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;
        // Write the entries to the file.
        json_compilation_db::write(BufWriter::new(file), entries)?;
        // Return the temporary file name.
        Ok(file_name)
    }

    /// Read the compilation database from a file.
    fn read_from_compilation_db(source: &Path) -> anyhow::Result<impl Iterator<Item = Entry>> {
        let file = OpenOptions::new()
            .read(true)
            .open(source)
            .with_context(|| format!("Failed to open file: {:?}", source))?;
        let entries = json_compilation_db::read(BufReader::new(file))
            .flat_map(Self::failed_entry_read_logged);

        Ok(entries)
    }

    fn failed_entry_read_logged(candidate: std::result::Result<Entry, Error>) -> Option<Entry> {
        match candidate {
            Ok(entry) => Some(entry),
            Err(error) => {
                // FIXME: write the file name to the log.
                log::error!("Failed to read entry: {}", error);
                None
            }
        }
    }
}

/// Validate the configuration of the output writer.
///
/// Validation is always successful, but it may modify the configuration values.
fn validate_filter(filter: &config::Filter) -> config::Filter {
    config::Filter {
        source: filter.source.clone(),
        compilers: filter.compilers.clone(),
        duplicates: config::DuplicateFilter {
            by_fields: validate_duplicates_by_fields(
                filter.duplicates.by_fields.as_slice(),
            ),
        },
    }
}

/// Validate the fields of the configuration.
///
/// Removes the duplicates from the list of fields.
fn validate_duplicates_by_fields(fields: &[config::OutputFields]) -> Vec<config::OutputFields> {
    fields
        .into_iter()
        .map(|field| field.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

impl TryFrom<&config::Filter> for filter::EntryPredicate {
    type Error = anyhow::Error;

    /// Create a filter from the configuration.
    fn try_from(config: &config::Filter) -> std::result::Result<Self, Self::Error> {
        // - Check if the source file exists
        // - Check if the source file is not in the exclude list of the configuration
        // - Check if the source file is in the include list of the configuration
        let source_exist_check = filter::EntryPredicateBuilder::filter_by_source_existence(
            config.source.include_only_existing_files,
        );
        let source_paths_to_exclude = filter::EntryPredicateBuilder::filter_by_source_paths(
            config.source.paths_to_exclude.clone(),
        );
        let source_paths_to_include = filter::EntryPredicateBuilder::filter_by_source_paths(
            config.source.paths_to_include.clone(),
        );
        let source_checks = source_exist_check & !source_paths_to_exclude & source_paths_to_include;
        // - Check if the compiler path is not in the list of the configuration
        // - Check if the compiler arguments are not in the list of the configuration
        let compiler_with_path = filter::EntryPredicateBuilder::filter_by_compiler_paths(
            config.compilers.with_paths.clone(),
        );
        let compiler_with_argument = filter::EntryPredicateBuilder::filter_by_compiler_arguments(
            config.compilers.with_arguments.clone(),
        );
        let compiler_checks = !compiler_with_path & !compiler_with_argument;
        // - Check if the entry is not a duplicate based on the fields of the configuration
        let hash_function = create_hash(config.duplicates.by_fields.clone());
        let duplicates = filter::EntryPredicateBuilder::filter_duplicate_entries(hash_function);

        Ok((source_checks & compiler_checks & duplicates).build())
    }
}

fn create_hash(fields: Vec<config::OutputFields>) -> impl Fn(&Entry) -> u64 + 'static {
    move |entry: &Entry| {
        let mut hasher = DefaultHasher::new();
        for field in &fields {
            match field {
                config::OutputFields::Directory => entry.directory.hash(&mut hasher),
                config::OutputFields::File => entry.file.hash(&mut hasher),
                config::OutputFields::Arguments => entry.arguments.hash(&mut hasher),
                config::OutputFields::Output => entry.output.hash(&mut hasher),
            }
        }
        hasher.finish()
    }
}

pub fn into_entries(value: Meaning) -> Result<Vec<Entry>, anyhow::Error> {
    match value {
        Meaning::Compiler {
            compiler,
            working_dir,
            passes,
        } => {
            let entries = passes
                .iter()
                .flat_map(|pass| -> Result<Entry, anyhow::Error> {
                    match pass {
                        CompilerPass::Preprocess => {
                            Err(anyhow!("preprocess pass should not show up in results"))
                        }
                        CompilerPass::Compile {
                            source,
                            output,
                            flags,
                        } => Ok(Entry {
                            file: into_abspath(source.clone(), working_dir.as_path())?,
                            directory: working_dir.clone(),
                            output: into_abspath_opt(output.clone(), working_dir.as_path())?,
                            arguments: into_arguments(&compiler, source, output, flags)?,
                        }),
                    }
                })
                .collect();

            Ok(entries)
        }
        _ => Ok(vec![]),
    }
}

fn into_arguments(
    compiler: &PathBuf,
    source: &PathBuf,
    output: &Option<PathBuf>,
    flags: &Vec<String>,
) -> Result<Vec<String>, anyhow::Error> {
    let mut arguments: Vec<String> = vec![];
    // Assemble the arguments as it would be for a single source file.
    arguments.push(into_string(&compiler)?);
    for flag in flags {
        arguments.push(flag.clone());
    }
    if let Some(file) = output {
        arguments.push(String::from("-o"));
        arguments.push(into_string(file)?)
    }
    arguments.push(into_string(source)?);
    Ok(arguments)
}

fn into_abspath(path: PathBuf, root: &Path) -> Result<PathBuf, std::io::Error> {
    let candidate = if path.is_absolute() {
        path.absolutize()
    } else {
        path.absolutize_from(root)
    };
    candidate.map(|x| x.to_path_buf())
}

fn into_abspath_opt(path: Option<PathBuf>, root: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    path.map(|v| into_abspath(v, root)).transpose()
}

fn into_string(path: &Path) -> Result<String> {
    path.to_path_buf()
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow!("Path can't be encoded to UTF"))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::vec_of_strings;

    #[test]
    fn test_non_compilations() -> Result<()> {
        let empty: Vec<Entry> = vec![];

        let result: Vec<Entry> = into_entries(Meaning::Ignored)?;
        assert_eq!(empty, result);

        let input = Meaning::Compiler {
            compiler: PathBuf::from("/usr/bin/cc"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![],
        };
        let result: Vec<Entry> = into_entries(input)?;
        assert_eq!(empty, result);

        Ok(())
    }

    #[test]
    fn test_single_source_compilation() -> Result<()> {
        let input = Meaning::Compiler {
            compiler: PathBuf::from("clang"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![CompilerPass::Compile {
                source: PathBuf::from("source.c"),
                output: Some(PathBuf::from("source.o")),
                flags: vec_of_strings!["-Wall"],
            }],
        };

        let expected = vec![Entry {
            directory: PathBuf::from("/home/user"),
            file: PathBuf::from("/home/user/source.c"),
            arguments: vec_of_strings!["clang", "-Wall", "-o", "source.o", "source.c"],
            output: Some(PathBuf::from("/home/user/source.o")),
        }];

        let result: Vec<Entry> = into_entries(input)?;

        assert_eq!(expected, result);

        Ok(())
    }

    #[test]
    fn test_multiple_sources_compilation() -> Result<()> {
        let input = Meaning::Compiler {
            compiler: PathBuf::from("clang"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![
                CompilerPass::Preprocess,
                CompilerPass::Compile {
                    source: PathBuf::from("/tmp/source1.c"),
                    output: None,
                    flags: vec_of_strings!["-Wall"],
                },
                CompilerPass::Compile {
                    source: PathBuf::from("../source2.c"),
                    output: None,
                    flags: vec_of_strings!["-Wall"],
                },
            ],
        };

        let expected = vec![
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/tmp/source1.c"),
                arguments: vec_of_strings!["clang", "-Wall", "/tmp/source1.c"],
                output: None,
            },
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/home/source2.c"),
                arguments: vec_of_strings!["clang", "-Wall", "../source2.c"],
                output: None,
            },
        ];

        let result: Vec<Entry> = into_entries(input)?;

        assert_eq!(expected, result);

        Ok(())
    }
}

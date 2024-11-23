// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use super::{args, config, semantic};
use anyhow::{anyhow, Context, Result};
use path_absolutize::Absolutize;

mod clang;
mod filter;

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
    pub fn configure(args: &args::BuildSemantic, config: &config::Output) -> Result<Self> {
        match config {
            config::Output::Clang { format, filter, .. } => {
                let result = OutputWriter {
                    output: PathBuf::from(&args.file_name),
                    append: args.append,
                    filter: filter.clone(),
                    format: format.clone(),
                };
                Ok(result)
            }
            config::Output::Semantic { .. } => {
                todo!("implement this case")
            }
        }
    }

    /// Implements the main logic of the output writer.
    pub fn run(&self, compiler_calls: impl Iterator<Item = semantic::CompilerCall>) -> Result<()> {
        let entries = compiler_calls
            .flat_map(|compiler_call| self.format.convert_into_entries(compiler_call));
        if self.append && self.output.exists() {
            let entries_from_db = Self::read_from_compilation_db(self.output.as_path())?;
            let final_entries = entries.chain(entries_from_db);
            self.write_into_compilation_db(final_entries)
        } else {
            if self.append {
                log::warn!("The output file does not exist, the append option is ignored.");
            }
            self.write_into_compilation_db(entries)
        }
    }

    fn write_into_compilation_db(&self, entries: impl Iterator<Item = clang::Entry>) -> Result<()> {
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
        entries: impl Iterator<Item = clang::Entry>,
    ) -> Result<PathBuf> {
        // Generate a temporary file name.
        let file_name = self.output.with_extension("tmp");
        // Open the file for writing.
        let file = File::create(&file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;
        // Write the entries to the file.
        self.format
            .write_entries(file, entries)
            .with_context(|| format!("Failed to write entries: {:?}", file_name.as_path()))?;
        // Return the temporary file name.
        Ok(file_name)
    }

    /// Read the compilation database from a file.
    fn read_from_compilation_db(source: &Path) -> Result<impl Iterator<Item = clang::Entry>> {
        let source_copy = source.to_path_buf();

        let file = OpenOptions::new()
            .read(true)
            .open(source)
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = clang::read(file)
            .map(move |candidate| {
                // We are here to log the error.
                candidate.map_err(|error| {
                    log::error!("Failed to read file: {:?}, reason: {}", source_copy, error);
                    error
                })
            })
            .filter_map(Result::ok);
        Ok(entries)
    }
}

impl config::Format {
    /// The entries are written in the format specified by the configuration.
    fn write_entries(
        &self,
        writer: impl std::io::Write,
        entries: impl Iterator<Item = clang::Entry>,
    ) -> Result<()> {
        let method = if self.command_as_array {
            clang::write_with_arguments
        } else {
            clang::write_with_command
        };
        method(writer, entries)?;

        Ok(())
    }

    /// Convert the compiler calls into entries.
    ///
    /// The conversion is done by converting the compiler passes into entries.
    /// Errors are logged and ignored. The entries format is controlled by the configuration.
    fn convert_into_entries(&self, compiler_call: semantic::CompilerCall) -> Vec<clang::Entry> {
        let semantic::CompilerCall {
            compiler,
            working_dir,
            passes,
        } = compiler_call;
        let entries = passes
            .iter()
            .map(|pass| self.try_convert_from_pass(&working_dir, &compiler, pass))
            // We are here to log the error.
            .map(|result| result.map_err(|error| log::info!("{}", error)))
            .filter_map(Result::ok)
            .collect();
        entries
    }

    /// Creates a single entry from a compiler pass if possible.
    ///
    /// The preprocess pass is ignored, and the compile pass is converted into an entry.
    ///
    /// Setting the file and output fields to use fully qualified paths. The reason for
    /// that is to make the compilation database independent of the working directory.
    /// FIXME: can be ^ this configurable?
    fn try_convert_from_pass(
        &self,
        working_dir: &Path,
        compiler: &Path,
        pass: &semantic::CompilerPass,
    ) -> Result<clang::Entry> {
        match pass {
            semantic::CompilerPass::Preprocess => {
                Err(anyhow!("preprocess pass should not show up in results"))
            }
            semantic::CompilerPass::Compile {
                source,
                output,
                flags,
            } => {
                let entry = clang::Entry {
                    file: into_fully_qualified_path(source.clone(), working_dir)?,
                    directory: working_dir.to_path_buf(),
                    output: self.try_convert_to_output(output, working_dir)?,
                    arguments: Self::try_convert_into_arguments(compiler, source, output, flags)?,
                };
                Ok(entry)
            }
        }
    }

    /// Convert the output path into a fully qualified path.
    ///
    /// If the output field is dropped, then the output is set to None.
    /// Otherwise, the output path is converted into a fully qualified path,
    /// based on the working directory.
    fn try_convert_to_output(
        &self,
        path: &Option<PathBuf>,
        root: &Path,
    ) -> Result<Option<PathBuf>, std::io::Error> {
        if self.drop_output_field {
            Ok(None)
        } else {
            path.clone()
                .map(|v| into_fully_qualified_path(v, root))
                .transpose()
        }
    }

    /// Reconstruct the arguments for the compiler call.
    ///
    /// It is not the same as the command line arguments, because the compiler call is
    /// decomposed into a separate lists of arguments. To assemble from the parts will
    /// not necessarily result in the same command line arguments. One example for that
    /// is the multiple source files are treated as separate compiler calls. Another
    /// thing that can change is the order of the arguments.
    fn try_convert_into_arguments(
        compiler: &Path,
        source: &Path,
        output: &Option<PathBuf>,
        flags: &Vec<String>,
    ) -> Result<Vec<String>, anyhow::Error> {
        let mut arguments: Vec<String> = vec![];
        // Assemble the arguments as it would be for a single source file.
        arguments.push(into_string(compiler)?);
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
}

// TODO: can this return Cow<Path>?
fn into_fully_qualified_path(path: PathBuf, root: &Path) -> Result<PathBuf, std::io::Error> {
    let candidate = if path.is_absolute() {
        path.absolutize()
    } else {
        path.absolutize_from(root)
    };
    candidate.map(|x| x.to_path_buf())
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
    fn test_non_compilations() {
        let format = config::Format {
            command_as_array: true,
            drop_output_field: false,
        };

        let expected: Vec<clang::Entry> = vec![];

        let input = semantic::CompilerCall {
            compiler: PathBuf::from("/usr/bin/cc"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![semantic::CompilerPass::Preprocess],
        };

        let result = format.convert_into_entries(input);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_single_source_compilation() {
        let format = config::Format {
            command_as_array: true,
            drop_output_field: false,
        };

        let input = semantic::CompilerCall {
            compiler: PathBuf::from("clang"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![semantic::CompilerPass::Compile {
                source: PathBuf::from("source.c"),
                output: Some(PathBuf::from("source.o")),
                flags: vec_of_strings!["-Wall"],
            }],
        };

        let expected = vec![clang::Entry {
            directory: PathBuf::from("/home/user"),
            file: PathBuf::from("/home/user/source.c"),
            arguments: vec_of_strings!["clang", "-Wall", "-o", "source.o", "source.c"],
            output: Some(PathBuf::from("/home/user/source.o")),
        }];

        let result = format.convert_into_entries(input);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_multiple_sources_compilation() {
        let format = config::Format {
            command_as_array: true,
            drop_output_field: true,
        };

        let input = semantic::CompilerCall {
            compiler: PathBuf::from("clang"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![
                semantic::CompilerPass::Preprocess,
                semantic::CompilerPass::Compile {
                    source: PathBuf::from("/tmp/source1.c"),
                    output: Some(PathBuf::from("./source1.o")),
                    flags: vec_of_strings![],
                },
                semantic::CompilerPass::Compile {
                    source: PathBuf::from("../source2.c"),
                    output: None,
                    flags: vec_of_strings!["-Wall"],
                },
            ],
        };

        let expected = vec![
            clang::Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/tmp/source1.c"),
                arguments: vec_of_strings!["clang", "-o", "./source1.o", "/tmp/source1.c"],
                output: None,
            },
            clang::Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/home/source2.c"),
                arguments: vec_of_strings!["clang", "-Wall", "../source2.c"],
                output: None,
            },
        ];

        let result = format.convert_into_entries(input);
        assert_eq!(expected, result);
    }
}

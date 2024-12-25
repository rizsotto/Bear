// SPDX-License-Identifier: GPL-3.0-or-later

use crate::intercept::Envelope;
use crate::output::OutputWriter;
use crate::semantic::interpreters::Builder;
use crate::semantic::transformation::Transformation;
use crate::{args, config, output, semantic};
use anyhow::{anyhow, Context};
use path_absolutize::Absolutize;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// The semantic analysis that is independent of the event source.
pub(super) struct SemanticAnalysisPipeline {
    interpreter: Box<dyn semantic::Interpreter>,
    transform: Box<dyn semantic::Transform>,
    output_writer: OutputWriterImpl,
}

impl SemanticAnalysisPipeline {
    /// Create a new semantic mode instance.
    pub(super) fn from(output: args::BuildSemantic, config: &config::Main) -> anyhow::Result<Self> {
        let interpreter = Builder::from(config);
        let transform = Transformation::from(&config.output);
        let output_writer = OutputWriterImpl::create(&output, &config.output)?;

        Ok(Self {
            interpreter: Box::new(interpreter),
            transform: Box::new(transform),
            output_writer,
        })
    }

    /// Consumer the envelopes for analysis and write the result to the output file.
    /// This implements the pipeline of the semantic analysis.
    pub(super) fn analyze_and_write(
        self,
        envelopes: impl IntoIterator<Item = Envelope>,
    ) -> anyhow::Result<()> {
        // Set up the pipeline of compilation database entries.
        let entries = envelopes
            .into_iter()
            .inspect(|envelope| log::debug!("envelope: {}", envelope))
            .map(|envelope| envelope.event.execution)
            .flat_map(|execution| self.interpreter.recognize(&execution))
            .inspect(|semantic| log::debug!("semantic: {:?}", semantic))
            .flat_map(|semantic| self.transform.apply(semantic));
        // Consume the entries and write them to the output file.
        // The exit code is based on the result of the output writer.
        self.output_writer.run(entries)
    }
}

/// The output writer implementation.
///
/// This is a workaround for the lack of trait object support for generic arguments.
/// https://doc.rust-lang.org/reference/items/traits.html#object-safety.
pub(crate) enum OutputWriterImpl {
    Clang(ClangOutputWriter),
    Semantic(SemanticOutputWriter),
}

impl OutputWriter for OutputWriterImpl {
    fn run(
        &self,
        compiler_calls: impl Iterator<Item = semantic::CompilerCall>,
    ) -> anyhow::Result<()> {
        match self {
            OutputWriterImpl::Clang(writer) => writer.run(compiler_calls),
            OutputWriterImpl::Semantic(writer) => writer.run(compiler_calls),
        }
    }
}

impl OutputWriterImpl {
    /// Create a new instance of the output writer.
    pub(crate) fn create(
        args: &args::BuildSemantic,
        config: &config::Output,
    ) -> anyhow::Result<OutputWriterImpl> {
        match config {
            config::Output::Clang { format, filter, .. } => {
                let result = ClangOutputWriter {
                    output: PathBuf::from(&args.file_name),
                    append: args.append,
                    filter: filter.clone(),
                    format: format.clone(),
                };
                Ok(OutputWriterImpl::Clang(result))
            }
            config::Output::Semantic { .. } => {
                let result = SemanticOutputWriter {
                    output: PathBuf::from(&args.file_name),
                };
                Ok(OutputWriterImpl::Semantic(result))
            }
        }
    }
}

pub(crate) struct SemanticOutputWriter {
    output: PathBuf,
}

impl OutputWriter for SemanticOutputWriter {
    fn run(&self, entries: impl Iterator<Item = semantic::CompilerCall>) -> anyhow::Result<()> {
        let file_name = &self.output;
        let file = File::create(file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;

        semantic::serialize(file, entries)?;

        Ok(())
    }
}

/// Responsible for writing the final compilation database file.
///
/// Implements filtering, formatting and atomic file writing.
/// (Atomic file writing implemented by writing to a temporary file and renaming it.)
///
/// Filtering is implemented by the `filter` module, and the formatting is implemented by the
/// `json_compilation_db` module.
pub(crate) struct ClangOutputWriter {
    output: PathBuf,
    append: bool,
    filter: config::Filter,
    format: config::Format,
}

impl OutputWriter for ClangOutputWriter {
    /// Implements the main logic of the output writer.
    fn run(
        &self,
        compiler_calls: impl Iterator<Item = semantic::CompilerCall>,
    ) -> anyhow::Result<()> {
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
}

impl ClangOutputWriter {
    /// Write the entries to the compilation database.
    ///
    /// The entries are written to a temporary file and then renamed to the final output.
    /// This guaranties that the output file is always in a consistent state.
    fn write_into_compilation_db(
        &self,
        entries: impl Iterator<Item = crate::output::clang::Entry>,
    ) -> anyhow::Result<()> {
        // Filter out the entries as per the configuration.
        let filter: crate::output::filter::EntryPredicate = TryFrom::try_from(&self.filter)?;
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
        entries: impl Iterator<Item = crate::output::clang::Entry>,
    ) -> anyhow::Result<PathBuf> {
        // Generate a temporary file name.
        let file_name = self.output.with_extension("tmp");
        // Open the file for writing.
        let file = File::create(&file_name)
            .map(BufWriter::new)
            .with_context(|| format!("Failed to create file: {:?}", file_name.as_path()))?;
        // Write the entries to the file.
        output::clang::write(self.format.command_as_array, file, entries)
            .with_context(|| format!("Failed to write entries: {:?}", file_name.as_path()))?;
        // Return the temporary file name.
        Ok(file_name)
    }

    /// Read the compilation database from a file.
    fn read_from_compilation_db(
        source: &Path,
    ) -> anyhow::Result<impl Iterator<Item = crate::output::clang::Entry>> {
        let source_copy = source.to_path_buf();

        let file = OpenOptions::new()
            .read(true)
            .open(source)
            .map(BufReader::new)
            .with_context(|| format!("Failed to open file: {:?}", source))?;

        let entries = crate::output::clang::read(file)
            .map(move |candidate| {
                // We are here to log the error.
                candidate.map_err(|error| {
                    log::error!("Failed to read file: {:?}, reason: {}", source_copy, error);
                    error
                })
            })
            .filter_map(anyhow::Result::ok);
        Ok(entries)
    }
}

impl config::Format {
    /// Convert the compiler calls into entries.
    ///
    /// The conversion is done by converting the compiler passes into entries.
    /// Errors are logged and ignored. The entries format is controlled by the configuration.
    fn convert_into_entries(
        &self,
        compiler_call: semantic::CompilerCall,
    ) -> Vec<crate::output::clang::Entry> {
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
            .filter_map(anyhow::Result::ok)
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
    ) -> anyhow::Result<crate::output::clang::Entry> {
        match pass {
            semantic::CompilerPass::Preprocess => {
                Err(anyhow!("preprocess pass should not show up in results"))
            }
            semantic::CompilerPass::Compile {
                source,
                output,
                flags,
            } => {
                let entry = crate::output::clang::Entry {
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
    ) -> anyhow::Result<Option<PathBuf>, std::io::Error> {
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
    ) -> anyhow::Result<Vec<String>, anyhow::Error> {
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
fn into_fully_qualified_path(
    path: PathBuf,
    root: &Path,
) -> anyhow::Result<PathBuf, std::io::Error> {
    let candidate = if path.is_absolute() {
        path.absolutize()
    } else {
        path.absolutize_from(root)
    };
    candidate.map(|x| x.to_path_buf())
}

fn into_string(path: &Path) -> anyhow::Result<String> {
    path.to_path_buf()
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow!("Path can't be encoded to UTF"))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::output::clang;
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

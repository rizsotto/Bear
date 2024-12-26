// SPDX-License-Identifier: GPL-3.0-or-later

use crate::output::clang::Entry;
use crate::{config, semantic};
use anyhow::anyhow;
use path_absolutize::Absolutize;
use std::path::{Path, PathBuf};

pub struct EntryFormatter {
    drop_output_field: bool,
}

impl From<&config::Format> for EntryFormatter {
    /// Create a formatter from the configuration.
    fn from(config: &config::Format) -> Self {
        let drop_output_field = config.drop_output_field;

        Self { drop_output_field }
    }
}

impl EntryFormatter {
    /// Convert the compiler calls into entries.
    ///
    /// The conversion is done by converting the compiler passes into entries.
    /// Errors are logged and ignored. The entries format is controlled by the configuration.
    pub(crate) fn apply(&self, compiler_call: semantic::CompilerCall) -> Vec<Entry> {
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
    ) -> anyhow::Result<Entry> {
        match pass {
            semantic::CompilerPass::Preprocess => {
                Err(anyhow!("preprocess pass should not show up in results"))
            }
            semantic::CompilerPass::Compile {
                source,
                output,
                flags,
            } => Ok(Entry {
                file: into_fully_qualified_path(source.clone(), working_dir)?,
                directory: working_dir.to_path_buf(),
                output: self.try_convert_to_output(output, working_dir)?,
                arguments: Self::try_convert_into_arguments(compiler, source, output, flags)?,
            }),
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
    use crate::vec_of_strings;

    #[test]
    fn test_non_compilations() {
        let format = config::Format {
            command_as_array: true,
            drop_output_field: false,
        };

        let expected: Vec<Entry> = vec![];

        let input = semantic::CompilerCall {
            compiler: PathBuf::from("/usr/bin/cc"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![semantic::CompilerPass::Preprocess],
        };

        let sut: EntryFormatter = (&format).into();
        let result = sut.apply(input);
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

        let expected = vec![Entry {
            directory: PathBuf::from("/home/user"),
            file: PathBuf::from("/home/user/source.c"),
            arguments: vec_of_strings!["clang", "-Wall", "-o", "source.o", "source.c"],
            output: Some(PathBuf::from("/home/user/source.o")),
        }];

        let sut: EntryFormatter = (&format).into();
        let result = sut.apply(input);
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
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/tmp/source1.c"),
                arguments: vec_of_strings!["clang", "-o", "./source1.o", "/tmp/source1.c"],
                output: None,
            },
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/home/source2.c"),
                arguments: vec_of_strings!["clang", "-Wall", "../source2.c"],
                output: None,
            },
        ];

        let sut: EntryFormatter = (&format).into();
        let result = sut.apply(input);
        assert_eq!(expected, result);
    }
}

// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::interpreters::generic::{CompilerCall, CompilerPass};
use super::Entry;
use anyhow::anyhow;
use std::path::{Path, PathBuf};

pub struct EntryConverter {}

impl EntryConverter {
    pub fn new() -> Self {
        Self {}
    }

    /// Convert the compiler calls into entries.
    ///
    /// The conversion is done by converting the compiler passes into entries.
    /// Errors are logged and ignored. The entries format is controlled by the configuration.
    pub fn apply(&self, command: CompilerCall) -> Vec<Entry> {
        let CompilerCall {
            compiler,
            working_dir,
            passes,
        } = command;
        passes
            .into_iter()
            .map(|pass| Self::try_convert_from_pass(&working_dir, &compiler, pass))
            // We are here to log the error.
            .map(|result| result.map_err(|error| log::info!("{}", error)))
            .filter_map(Result::ok)
            .collect()
    }

    /// Creates a single entry from a compiler pass if possible.
    ///
    /// The preprocess pass is ignored, and the compile pass is converted into an entry.
    ///
    /// The file and directory paths are converted into fully qualified paths when required.
    fn try_convert_from_pass(
        working_dir: &Path,
        compiler: &Path,
        pass: CompilerPass,
    ) -> anyhow::Result<Entry> {
        match pass {
            CompilerPass::Preprocess => {
                Err(anyhow!("preprocess pass should not show up in results"))
            }
            CompilerPass::Compile {
                source,
                output,
                flags,
            } => {
                let entry = Entry::from_arguments(
                    &source,
                    Self::arguments(compiler, &source, &flags, output.as_ref())?,
                    working_dir,
                    output,
                );

                entry.validate()?;

                Ok(entry)
            }
        }
    }

    /// Reconstruct the arguments for the compiler call.
    ///
    /// It is not the same as the command line arguments, because the compiler call is
    /// decomposed into a separate lists of arguments. To assemble from the parts will
    /// not necessarily result in the same command line arguments. One example for that
    /// is the multiple source files are treated as separate compiler calls. Another
    /// thing that can change is the order of the arguments.
    fn arguments(
        compiler: &Path,
        source: &Path,
        flags: &[String],
        output: Option<&PathBuf>,
    ) -> anyhow::Result<Vec<String>> {
        let mut arguments = Vec::with_capacity(flags.len() + 3);
        arguments.push(into_string(compiler)?);
        arguments.extend(flags.iter().cloned());
        if let Some(file) = output {
            arguments.push("-o".to_string());
            arguments.push(into_string(file)?);
        }
        arguments.push(into_string(source)?);
        Ok(arguments)
    }
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

    #[test]
    fn test_non_compilations() {
        let input = CompilerCall {
            compiler: "/usr/bin/cc".into(),
            working_dir: "/home/user".into(),
            passes: vec![CompilerPass::Preprocess],
        };

        let sut = EntryConverter::new();
        let result = sut.apply(input);

        let expected: Vec<Entry> = vec![];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_single_source_compilation() {
        let input = CompilerCall {
            compiler: "/usr/bin/clang".into(),
            working_dir: "/home/user".into(),
            passes: vec![CompilerPass::Compile {
                source: "source.c".into(),
                output: Some("source.o".into()),
                flags: vec!["-Wall".into()],
            }],
        };

        let sut = EntryConverter::new();
        let result = sut.apply(input);

        let expected = vec![Entry::from_arguments_str(
            "source.c",
            vec!["/usr/bin/clang", "-Wall", "-o", "source.o", "source.c"],
            "/home/user",
            Some("source.o"),
        )];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_multiple_sources_compilation() {
        let input = CompilerCall {
            compiler: "clang".into(),
            working_dir: "/home/user".into(),
            passes: vec![
                CompilerPass::Preprocess,
                CompilerPass::Compile {
                    source: "/tmp/source1.c".into(),
                    output: Some("./source1.o".into()),
                    flags: vec![],
                },
                CompilerPass::Compile {
                    source: "../source2.c".into(),
                    output: None,
                    flags: vec!["-Wall".into()],
                },
            ],
        };

        let sut = EntryConverter::new();
        let result = sut.apply(input);

        let expected = vec![
            Entry::from_arguments_str(
                "/tmp/source1.c",
                vec!["clang", "-o", "./source1.o", "/tmp/source1.c"],
                "/home/user",
                Some("./source1.o"),
            ),
            Entry::from_arguments_str(
                "../source2.c",
                vec!["clang", "-Wall", "../source2.c"],
                "/home/user",
                None,
            ),
        ];
        assert_eq!(expected, result);
    }
}

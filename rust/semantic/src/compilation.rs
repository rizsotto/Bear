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

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use json_compilation_db::Entry;
use path_absolutize::Absolutize;

use crate::result::{CompilerPass, Semantic};

impl TryFrom<Semantic> for Vec<Entry> {
    type Error = anyhow::Error;

    fn try_from(value: Semantic) -> Result<Self, Self::Error> {
        match value {
            Semantic::Compiler { compiler, working_dir, passes } => {
                let entries = passes.iter()
                    .flat_map(|pass| -> Result<Entry, Self::Error> {
                        match pass {
                            CompilerPass::Preprocess =>
                                Err(anyhow!("preprocess pass should not show up in results")),
                            CompilerPass::Compile { source, output, flags } =>
                                Ok(
                                    Entry {
                                        file: into_abspath(source.clone(), working_dir.as_path())?,
                                        directory: working_dir.clone(),
                                        output: into_abspath_opt(output.clone(), working_dir.as_path())?,
                                        arguments: into_arguments(&compiler, source, output, flags)?,
                                    }
                                )
                        }
                    })
                    .collect();

                Ok(entries)
            }
            _ =>
                Ok(vec![]),
        }
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
    path.map(|v| into_abspath(v, root))
        .transpose()
}

fn into_string(path: &Path) -> Result<String> {
    path.to_path_buf()
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow!("Path can't be encoded to UTF"))
}

#[cfg(test)]
mod test {
    use crate::vec_of_strings;

    use super::*;

    #[test]
    fn test_non_compilations() -> Result<()> {
        let empty: Vec<Entry> = vec![];

        let result: Vec<Entry> = Semantic::UnixCommand.try_into()?;
        assert_eq!(empty, result);
        let result: Vec<Entry> = Semantic::BuildCommand.try_into()?;
        assert_eq!(empty, result);

        let input = Semantic::Compiler {
            compiler: PathBuf::from("/usr/bin/cc"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![],
        };
        let result: Vec<Entry> = input.try_into()?;
        assert_eq!(empty, result);

        Ok(())
    }

    #[test]
    fn test_single_source_compilation() -> Result<()> {
        let input = Semantic::Compiler {
            compiler: PathBuf::from("clang"),
            working_dir: PathBuf::from("/home/user"),
            passes: vec![
                CompilerPass::Compile {
                    source: PathBuf::from("source.c"),
                    output: Some(PathBuf::from("source.o")),
                    flags: vec_of_strings!["-Wall"],
                },
            ],
        };

        let expected = vec![
            Entry {
                directory: PathBuf::from("/home/user"),
                file: PathBuf::from("/home/user/source.c"),
                arguments: vec_of_strings!["clang", "-Wall", "-o", "source.o", "source.c"],
                output: Some(PathBuf::from("/home/user/source.o")),
            }
        ];

        let result: Vec<Entry> = input.try_into()?;

        assert_eq!(expected, result);

        Ok(())
    }

    #[test]
    fn test_multiple_sources_compilation() -> Result<()> {
        let input = Semantic::Compiler {
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

        let result: Vec<Entry> = input.try_into()?;

        assert_eq!(expected, result);

        Ok(())
    }
}

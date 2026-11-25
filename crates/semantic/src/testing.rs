// SPDX-License-Identifier: GPL-3.0-or-later

use super::interpretation::Provenance;
use super::interpreters::matchers::source::file_type_of;
use super::{
    Arguments, FileRef, FileType, Interpretation, OutputSpelling, PassEffect, Recognized, Role, SourceMode,
};
use intercept::Execution;
use std::collections::HashMap;
use std::path::PathBuf;

/// What a test item stands for. A test lists `(Item, tokens)` pairs instead of
/// building an [`Interpretation`] by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Compiler,
    /// A file named on the command line. `binary: true` needs a name that is
    /// an object file.
    Source {
        binary: bool,
    },
    /// An output flag. Two tokens (`["-o", "a.o"]`) are the separate form, one
    /// token is the glued form.
    Output,
    /// An `@file` token that was not read.
    ArgumentsFile,
    Flag(PassEffect),
}

impl Interpretation {
    /// The call as `(item, tokens)` pairs in output order, without the
    /// launcher. The inverse of the list `Recognized::from_strings` takes.
    #[cfg(test)]
    pub(crate) fn items(&self) -> Vec<(Item, Vec<String>)> {
        self.arguments()
            .filter_map(|argument| {
                let item = match &argument.role {
                    Role::Launcher => return None,
                    Role::Compiler { .. } => Item::Compiler,
                    Role::Input(FileRef::Resolved { file_type, .. }) => {
                        Item::Source { binary: *file_type == FileType::Object }
                    }
                    Role::Input(FileRef::UnresolvedLibrary { .. }) => {
                        panic!("a library input has no test item: {:?}", argument)
                    }
                    Role::ArgumentsFile { .. } => Item::ArgumentsFile,
                    Role::Output { .. } => Item::Output,
                    Role::Flag(effect) => Item::Flag(*effect),
                };
                Some((item, argument.as_arguments(&|path| std::borrow::Cow::Borrowed(path))))
            })
            .collect()
    }

    /// Build an Interpretation from a list of test items.
    ///
    /// An input is typed by its extension, so a `Source { binary }` flag must
    /// agree with it: `binary: true` needs a name that is an object.
    fn from_strings(executable: &str, arguments: &[(Item, Vec<&str>)]) -> Self {
        let items = arguments
            .iter()
            .map(|(kind, args)| {
                let role = match kind {
                    Item::Compiler => Role::Compiler { path: PathBuf::from(executable) },
                    Item::Source { binary } => {
                        let file_type = file_type_of(args[0]);
                        assert_eq!(
                            *binary,
                            file_type == FileType::Object,
                            "the binary flag disagrees with the extension of {:?}",
                            args[0]
                        );
                        Role::Input(FileRef::Resolved { path: PathBuf::from(args[0]), file_type })
                    }
                    Item::Output => Role::Output {
                        flag: args[0].to_string(),
                        path: PathBuf::from(args.get(1).unwrap_or(&"")),
                        spelling: if args.len() > 1 {
                            OutputSpelling::Separate
                        } else {
                            OutputSpelling::Glued
                        },
                    },
                    Item::ArgumentsFile => Role::ArgumentsFile {
                        path: PathBuf::from(args[0].strip_prefix('@').unwrap_or(args[0])),
                    },
                    Item::Flag(effect) => Role::Flag(*effect),
                };
                let item = Arguments { args: args.iter().map(|arg| arg.to_string()).collect(), role };
                (Provenance::CommandLine, item)
            })
            .collect();
        Interpretation::new("test", SourceMode::PerSourceStripped, items)
    }
}

impl Recognized {
    /// Create a Recognized from string arguments for testing.
    ///
    /// The execution's argv is every argument token in order. Call
    /// [`with_source_mode`](Self::with_source_mode) to change the mode.
    pub fn from_strings(working_dir: &str, executable: &str, arguments: Vec<(Item, Vec<&str>)>) -> Self {
        let argv: Vec<&str> = arguments.iter().flat_map(|(_, tokens)| tokens.iter().copied()).collect();
        let execution = Execution::from_strings(executable, argv, working_dir, HashMap::new());
        let interpretation = Interpretation::from_strings(executable, &arguments);
        Self { execution, interpretation }
    }

    /// Set the source mode on the interpretation.
    pub fn with_source_mode(mut self, source_mode: SourceMode) -> Self {
        self.interpretation.set_source_mode(source_mode);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::path::Path;

    /// Each spelling writes back the tokens it stands for: re-joining a glued
    /// or `=`/`:` written value as two tokens would produce a command line the
    /// originating compiler rejects.
    #[test]
    fn output_spelling_writes_the_tokens_it_stands_for() {
        // arrange
        let cases = [
            (OutputSpelling::Separate, "-o", vec!["-o", "main.o"]),
            (OutputSpelling::Glued, "-o", vec!["-omain.o"]),
            (OutputSpelling::Equals, "--output_file", vec!["--output_file=main.o"]),
            (OutputSpelling::Colon, "/Fo", vec!["/Fo:main.o"]),
        ];

        for (spelling, flag, expected) in cases {
            // act
            let sut = spelling.as_arguments(flag, Path::new("main.o"));

            // assert
            assert_eq!(sut, expected, "case {:?}", spelling);
        }
    }

    /// The path stays individually addressable inside a glued token: the
    /// updater rewrites the path and the flag is left alone.
    #[test]
    fn output_argument_path_updater_reaches_a_glued_path() {
        // arrange
        let sut = Arguments {
            args: vec!["-omain.o".to_string()],
            role: Role::Output {
                flag: "-o".to_string(),
                path: PathBuf::from("main.o"),
                spelling: OutputSpelling::Glued,
            },
        };
        let path_updater: &dyn Fn(&Path) -> Cow<Path> =
            &|path: &Path| Cow::Owned(Path::new("/build").join(path));
        // Spelled with the platform's own separator: `Path::join` writes a
        // backslash on Windows, and the point of the test is where the path
        // lands, not how the platform spells it.
        let rewritten = Path::new("/build").join("main.o");

        // act
        let actual = sut.as_arguments(path_updater);

        // assert
        assert_eq!(actual, vec![format!("-o{}", rewritten.display())]);
    }
}

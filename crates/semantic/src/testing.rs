// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Argument, ArgumentKind, Command, OutputSpelling, SourceMode};
use std::path::PathBuf;

impl Command {
    /// Create a Command from string arguments for testing.
    ///
    /// An `ArgumentKind::Output` entry spelled as two tokens (`["-o", "a.o"]`)
    /// becomes the separate form; one token becomes the glued form, which is
    /// the spelling that token would re-emit as.
    pub fn from_strings(
        working_dir: &str,
        executable: &str,
        arguments: Vec<(ArgumentKind, Vec<&str>)>,
    ) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            executable: PathBuf::from(executable),
            source_mode: SourceMode::PerSourceStripped,
            arguments: arguments
                .into_iter()
                .map(|(kind, args)| match kind {
                    ArgumentKind::Source { binary } => Argument::Source { path: args[0].to_string(), binary },
                    ArgumentKind::Output => Argument::Output {
                        flag: args[0].to_string(),
                        path: args.get(1).unwrap_or(&"").to_string(),
                        spelling: if args.len() > 1 {
                            OutputSpelling::Separate
                        } else {
                            OutputSpelling::Glued
                        },
                    },
                    other_kind => Argument::Other {
                        arguments: args.into_iter().map(String::from).collect(),
                        kind: other_kind,
                    },
                })
                .collect(),
        }
    }

    /// Compare two CompilerCommands by their arguments for testing.
    pub fn has_same_arguments(&self, other: &Command) -> bool {
        self.arguments == other.arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PassEffect;
    use std::borrow::Cow;
    use std::path::Path;

    #[test]
    fn test_compiler_command_comparison() {
        let cmd1 = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd3 = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["other.c"]),
                (ArgumentKind::Output, vec!["-o", "other.o"]),
            ],
        );

        assert!(cmd1.has_same_arguments(&cmd2));
        assert!(!cmd1.has_same_arguments(&cmd3));
    }

    #[test]
    fn test_arguments_with_different_kinds() {
        let cmd1 = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["main.c"])],
        );

        let cmd2 =
            Command::from_strings("/home/user", "/usr/bin/gcc", vec![(ArgumentKind::Output, vec!["main.c"])]);

        assert!(!cmd1.has_same_arguments(&cmd2));
    }

    #[test]
    fn test_arguments_with_different_lengths() {
        let cmd1 = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = Command::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["main.c"])],
        );

        assert!(!cmd1.has_same_arguments(&cmd2));
    }

    #[test]
    fn test_argument_enum_implementations() {
        let source_arg = Argument::Source { path: "main.c".to_string(), binary: false };
        let output_arg = Argument::Output {
            flag: "-o".to_string(),
            path: "main.o".to_string(),
            spelling: OutputSpelling::Separate,
        };
        let other_arg = Argument::Other {
            arguments: vec!["-Wall".to_string()],
            kind: ArgumentKind::Other(PassEffect::None),
        };

        assert_eq!(source_arg.kind(), ArgumentKind::Source { binary: false });
        assert_eq!(output_arg.kind(), ArgumentKind::Output);
        assert_eq!(other_arg.kind(), ArgumentKind::Other(PassEffect::None));

        let path_updater: &dyn Fn(&Path) -> Cow<Path> = &|path: &Path| Cow::Borrowed(path);
        assert_eq!(source_arg.as_arguments(path_updater), vec!["main.c"]);
        assert_eq!(output_arg.as_arguments(path_updater), vec!["-o", "main.o"]);
        assert_eq!(other_arg.as_arguments(path_updater), vec!["-Wall"]);

        assert_eq!(source_arg.as_file(path_updater), Some(PathBuf::from("main.c")));
        assert_eq!(output_arg.as_file(path_updater), Some(PathBuf::from("main.o")));
        assert_eq!(other_arg.as_file(path_updater), None);
    }

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
        let sut = Argument::Output {
            flag: "-o".to_string(),
            path: "main.o".to_string(),
            spelling: OutputSpelling::Glued,
        };
        let path_updater: &dyn Fn(&Path) -> Cow<Path> =
            &|path: &Path| Cow::Owned(Path::new("/build").join(path));

        // act
        let actual = sut.as_arguments(path_updater);

        // assert
        assert_eq!(actual, vec!["-o/build/main.o"]);
        assert_eq!(sut.as_file(path_updater), Some(PathBuf::from("/build/main.o")));
    }
}

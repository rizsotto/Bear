// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Argument, ArgumentKind, CompilerCommand, PassEffect};
use std::path::PathBuf;

impl CompilerCommand {
    /// Create a CompilerCommand from string arguments for testing.
    pub fn from_strings(
        working_dir: &str,
        executable: &str,
        arguments: Vec<(ArgumentKind, Vec<&str>)>,
    ) -> Self {
        Self {
            working_dir: PathBuf::from(working_dir),
            executable: PathBuf::from(executable),
            arguments: arguments
                .into_iter()
                .map(|(kind, args)| match kind {
                    ArgumentKind::Source { binary } => Argument::Source { path: args[0].to_string(), binary },
                    ArgumentKind::Output => Argument::Output {
                        flag: args[0].to_string(),
                        path: args.get(1).unwrap_or(&"").to_string(),
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
    pub fn has_same_arguments(&self, other: &CompilerCommand) -> bool {
        self.arguments == other.arguments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::path::Path;

    #[test]
    fn test_compiler_command_comparison() {
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd3 = CompilerCommand::from_strings(
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
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["main.c"])],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Output, vec!["main.c"])],
        );

        assert!(!cmd1.has_same_arguments(&cmd2));
    }

    #[test]
    fn test_arguments_with_different_lengths() {
        let cmd1 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![
                (ArgumentKind::Source { binary: false }, vec!["main.c"]),
                (ArgumentKind::Output, vec!["-o", "main.o"]),
            ],
        );

        let cmd2 = CompilerCommand::from_strings(
            "/home/user",
            "/usr/bin/gcc",
            vec![(ArgumentKind::Source { binary: false }, vec!["main.c"])],
        );

        assert!(!cmd1.has_same_arguments(&cmd2));
    }

    #[test]
    fn test_argument_enum_implementations() {
        let source_arg = Argument::Source { path: "main.c".to_string(), binary: false };
        let output_arg = Argument::Output { flag: "-o".to_string(), path: "main.o".to_string() };
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
}

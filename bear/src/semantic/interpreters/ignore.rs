// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::path::PathBuf;

use crate::semantic::{Command, Execution, Interpreter};

const COREUTILS_MESSAGE: &str = "coreutils executable";
const COMPILER_MESSAGE: &str = "compiler specified in config to ignore";

/// A tool to ignore a command execution by executable name.
pub(super) struct IgnoreByPath {
    executables: HashSet<PathBuf>,
    reason: &'static str,
}

impl IgnoreByPath {
    pub(super) fn new() -> Self {
        let executables = COREUTILS_FILES.iter().map(PathBuf::from).collect();
        Self {
            executables,
            reason: COREUTILS_MESSAGE,
        }
    }

    pub(super) fn from(compilers: &[PathBuf]) -> Self {
        let executables = compilers.iter().cloned().collect();
        Self {
            executables,
            reason: COMPILER_MESSAGE,
        }
    }
}

impl Default for IgnoreByPath {
    fn default() -> Self {
        Self::new()
    }
}

/// A tool to ignore a command execution by arguments.
impl Interpreter for IgnoreByPath {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        if self.executables.contains(&execution.executable) {
            Some(Command::Ignored(self.reason))
        } else {
            None
        }
    }
}

const COREUTILS_FILES: [&str; 106] = [
    "/usr/bin/[",
    "/usr/bin/arch",
    "/usr/bin/b2sum",
    "/usr/bin/base32",
    "/usr/bin/base64",
    "/usr/bin/basename",
    "/usr/bin/basenc",
    "/usr/bin/cat",
    "/usr/bin/chcon",
    "/usr/bin/chgrp",
    "/usr/bin/chmod",
    "/usr/bin/chown",
    "/usr/bin/cksum",
    "/usr/bin/comm",
    "/usr/bin/cp",
    "/usr/bin/csplit",
    "/usr/bin/cut",
    "/usr/bin/date",
    "/usr/bin/dd",
    "/usr/bin/df",
    "/usr/bin/dir",
    "/usr/bin/dircolors",
    "/usr/bin/dirname",
    "/usr/bin/du",
    "/usr/bin/echo",
    "/usr/bin/env",
    "/usr/bin/expand",
    "/usr/bin/expr",
    "/usr/bin/factor",
    "/usr/bin/false",
    "/usr/bin/fmt",
    "/usr/bin/fold",
    "/usr/bin/groups",
    "/usr/bin/head",
    "/usr/bin/hostid",
    "/usr/bin/id",
    "/usr/bin/install",
    "/usr/bin/join",
    "/usr/bin/link",
    "/usr/bin/ln",
    "/usr/bin/logname",
    "/usr/bin/ls",
    "/usr/bin/md5sum",
    "/usr/bin/mkdir",
    "/usr/bin/mkfifo",
    "/usr/bin/mknod",
    "/usr/bin/mktemp",
    "/usr/bin/mv",
    "/usr/bin/nice",
    "/usr/bin/nl",
    "/usr/bin/nohup",
    "/usr/bin/nproc",
    "/usr/bin/numfmt",
    "/usr/bin/od",
    "/usr/bin/paste",
    "/usr/bin/pathchk",
    "/usr/bin/pinky",
    "/usr/bin/pr",
    "/usr/bin/printenv",
    "/usr/bin/printf",
    "/usr/bin/ptx",
    "/usr/bin/pwd",
    "/usr/bin/readlink",
    "/usr/bin/realpath",
    "/usr/bin/rm",
    "/usr/bin/rmdir",
    "/usr/bin/runcon",
    "/usr/bin/seq",
    "/usr/bin/sha1sum",
    "/usr/bin/sha224sum",
    "/usr/bin/sha256sum",
    "/usr/bin/sha384sum",
    "/usr/bin/sha512sum",
    "/usr/bin/shred",
    "/usr/bin/shuf",
    "/usr/bin/sleep",
    "/usr/bin/sort",
    "/usr/bin/split",
    "/usr/bin/stat",
    "/usr/bin/stdbuf",
    "/usr/bin/stty",
    "/usr/bin/sum",
    "/usr/bin/sync",
    "/usr/bin/tac",
    "/usr/bin/tail",
    "/usr/bin/tee",
    "/usr/bin/test",
    "/usr/bin/timeout",
    "/usr/bin/touch",
    "/usr/bin/tr",
    "/usr/bin/true",
    "/usr/bin/truncate",
    "/usr/bin/tsort",
    "/usr/bin/tty",
    "/usr/bin/uname",
    "/usr/bin/unexpand",
    "/usr/bin/uniq",
    "/usr/bin/unlink",
    "/usr/bin/users",
    "/usr/bin/vdir",
    "/usr/bin/wc",
    "/usr/bin/who",
    "/usr/bin/whoami",
    "/usr/bin/yes",
    "/usr/bin/make",
    "/usr/bin/gmake",
];

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_executions_are_ignored_by_executable_name() {
        let input = Execution::from_strings(
            "/usr/bin/ls",
            vec!["ls", "/home/user/build"],
            "/home/user",
            HashMap::new(),
        );
        let sut = IgnoreByPath::new();
        let result = sut.recognize(&input);

        assert!(result.is_some());
    }

    #[test]
    fn test_not_known_executables_are_not_recognized() {
        let input = Execution::from_strings(
            "/usr/bin/unknown",
            vec!["unknown", "/home/user/build"],
            "/home/user",
            HashMap::new(),
        );
        let sut = IgnoreByPath::new();
        let result = sut.recognize(&input);

        assert!(result.is_none());
    }
}

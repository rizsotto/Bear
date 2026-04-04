// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::ffi::OsString;

use crate::semantic::{Execution, Interpreter, RecognizeResult};

const COREUTILS_MESSAGE: &str = "coreutils executable";
const COMPILER_MESSAGE: &str = "compiler specified in config to ignore";

/// A tool to ignore a command execution by executable filename.
///
/// Matches against the filename component of the executable path,
/// so `/usr/bin/ls` and `/usr/local/bin/ls` both match `ls`.
pub(super) struct IgnoreByPath {
    filenames: HashSet<OsString>,
    reason: &'static str,
}

impl IgnoreByPath {
    pub(super) fn new() -> Self {
        let filenames = COREUTILS_FILES.iter().map(OsString::from).collect();
        Self { filenames, reason: COREUTILS_MESSAGE }
    }

    pub(super) fn from(compilers: impl IntoIterator<Item = impl AsRef<std::path::Path>>) -> Self {
        let filenames =
            compilers.into_iter().filter_map(|p| p.as_ref().file_name().map(OsString::from)).collect();
        Self { filenames, reason: COMPILER_MESSAGE }
    }
}

impl Default for IgnoreByPath {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter for IgnoreByPath {
    fn recognize(&self, execution: Execution) -> RecognizeResult {
        if execution.executable.file_name().is_some_and(|f| self.filenames.contains(f)) {
            return RecognizeResult::Ignored(self.reason);
        }
        RecognizeResult::NotRecognized(execution)
    }
}

const COREUTILS_FILES: [&str; 106] = [
    "[",
    "arch",
    "b2sum",
    "base32",
    "base64",
    "basename",
    "basenc",
    "cat",
    "chcon",
    "chgrp",
    "chmod",
    "chown",
    "cksum",
    "comm",
    "cp",
    "csplit",
    "cut",
    "date",
    "dd",
    "df",
    "dir",
    "dircolors",
    "dirname",
    "du",
    "echo",
    "env",
    "expand",
    "expr",
    "factor",
    "false",
    "fmt",
    "fold",
    "groups",
    "head",
    "hostid",
    "id",
    "install",
    "join",
    "link",
    "ln",
    "logname",
    "ls",
    "md5sum",
    "mkdir",
    "mkfifo",
    "mknod",
    "mktemp",
    "mv",
    "nice",
    "nl",
    "nohup",
    "nproc",
    "numfmt",
    "od",
    "paste",
    "pathchk",
    "pinky",
    "pr",
    "printenv",
    "printf",
    "ptx",
    "pwd",
    "readlink",
    "realpath",
    "rm",
    "rmdir",
    "runcon",
    "seq",
    "sha1sum",
    "sha224sum",
    "sha256sum",
    "sha384sum",
    "sha512sum",
    "shred",
    "shuf",
    "sleep",
    "sort",
    "split",
    "stat",
    "stdbuf",
    "stty",
    "sum",
    "sync",
    "tac",
    "tail",
    "tee",
    "test",
    "timeout",
    "touch",
    "tr",
    "true",
    "truncate",
    "tsort",
    "tty",
    "uname",
    "unexpand",
    "uniq",
    "unlink",
    "users",
    "vdir",
    "wc",
    "who",
    "whoami",
    "yes",
    "make",
    "gmake",
];

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

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
        assert!(matches!(sut.recognize(input), RecognizeResult::Ignored(_)));
    }

    #[test]
    fn test_executions_are_ignored_regardless_of_path_prefix() {
        let input = Execution::from_strings(
            "/usr/local/bin/ls",
            vec!["ls", "/home/user/build"],
            "/home/user",
            HashMap::new(),
        );
        let sut = IgnoreByPath::new();
        assert!(matches!(sut.recognize(input), RecognizeResult::Ignored(_)));
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
        assert!(matches!(sut.recognize(input), RecognizeResult::NotRecognized(_)));
    }

    #[test]
    fn test_compiler_ignore_matches_on_filename() {
        let compilers = vec![PathBuf::from("/usr/bin/gcc")];
        let sut = IgnoreByPath::from(&compilers);

        let input = Execution::from_strings(
            "/opt/toolchain/bin/gcc",
            vec!["gcc", "-c", "foo.c"],
            "/home/user",
            HashMap::new(),
        );
        assert!(matches!(sut.recognize(input), RecognizeResult::Ignored(_)));
    }
}

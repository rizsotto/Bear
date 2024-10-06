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

use std::collections::HashSet;
use std::ffi::OsString;

use lazy_static::lazy_static;

use super::super::{Meaning, RecognitionResult, Tool};
use intercept::ipc::Execution;

pub(crate) struct Unix {}

impl Unix {
    pub(crate) fn new() -> Box<dyn Tool> {
        Box::new(Unix {})
    }
}

impl Tool for Unix {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        if let Some(executable) = execution.executable.file_name() {
            if COREUTILS_FILES.contains(executable) {
                return RecognitionResult::Recognized(Ok(Meaning::Ignored));
            }
        }
        RecognitionResult::NotRecognized
    }
}

lazy_static! {
    static ref COREUTILS_FILES: HashSet<OsString> = {
        let files_paths = [
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
        ]
        .iter()
        .map(OsString::from);

        HashSet::from_iter(files_paths)
    };
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::vec_of_strings;

    use super::*;

    #[test]
    fn test_unix_tools_are_recognized() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/ls"),
            arguments: vec_of_strings!["ls", "/home/user/build"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(
            RecognitionResult::Recognized(Ok(Meaning::Ignored)),
            SUT.recognize(&input)
        )
    }

    #[test]
    fn test_not_known_executables_are_not_recognized() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/bear"),
            arguments: vec_of_strings!["bear", "--", "make"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(RecognitionResult::NotRecognized, SUT.recognize(&input))
    }

    lazy_static! {
        static ref SUT: Unix = Unix {};
    }
}

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
use std::path::PathBuf;

use super::super::{Meaning, RecognitionResult, Tool};
use intercept::ipc::Execution;

pub struct IgnoreByPath {
    executables: HashSet<PathBuf>,
}

impl IgnoreByPath {
    pub fn new() -> Box<dyn Tool> {
        let executables = COREUTILS_FILES.iter().map(PathBuf::from).collect();
        Box::new(Self { executables })
    }

    pub fn from(compilers: &[PathBuf]) -> Box<dyn Tool> {
        let executables = compilers.iter().map(|compiler| compiler.clone()).collect();
        Box::new(Self { executables })
    }
}

impl Tool for IgnoreByPath {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        if self.executables.contains(&execution.executable) {
            RecognitionResult::Recognized(Ok(Meaning::Ignored))
        } else {
            RecognitionResult::NotRecognized
        }
    }
}

pub struct IgnoreByArgs {
    args: Vec<String>,
}

impl IgnoreByArgs {
    pub fn new(args: &[String]) -> Box<dyn Tool> {
        let clones = args.iter().map(|arg| arg.clone()).collect();
        Box::new(Self { args: clones })
    }
}

impl Tool for IgnoreByArgs {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        if execution
            .arguments
            .iter()
            .any(|arg| self.args.contains(arg))
        {
            RecognitionResult::Recognized(Ok(Meaning::Ignored))
        } else {
            RecognitionResult::NotRecognized
        }
    }
}

static COREUTILS_FILES: [&str; 106] = [
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
        let sut = IgnoreByPath::new();

        assert_eq!(
            RecognitionResult::Recognized(Ok(Meaning::Ignored)),
            sut.recognize(&input)
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
        let sut = IgnoreByPath::new();

        assert_eq!(RecognitionResult::NotRecognized, sut.recognize(&input))
    }

    // TODO: implement test cases for args
}

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
use std::path::Path;

use lazy_static::lazy_static;

use intercept::ipc::Execution;
use crate::tools::{RecognitionResult, Meaning, Tool};

pub(crate) struct Unix {}

impl Unix {
    pub(crate) fn new() -> Box<dyn Tool> {
        Box::new(Unix {})
    }
}

impl Tool for Unix {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        let executable = execution.executable.as_path();
        if COREUTILS_FILES.contains(executable) {
            RecognitionResult::Recognized(Ok(Meaning::Ignored))
        } else {
            RecognitionResult::NotRecognized
        }
    }
}

lazy_static! {
    static ref COREUTILS_FILES: HashSet<&'static Path> = {
		let files_paths = [
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
		]
			.iter()
			.map(Path::new);

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

	lazy_static! {
        static ref SUT: Unix = Unix {};
    }
}
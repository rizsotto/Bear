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

use crate::execution::Execution;
use crate::tools::{RecognitionResult, Semantic, Tool};

pub(crate) struct Build {}

impl Build {
    pub(crate) fn new() -> Box<dyn Tool> {
        Box::new(Build {})
    }
}

impl Tool for Build {
    fn recognize(&self, execution: &Execution) -> RecognitionResult {
        let executable = execution.executable.as_path();
        if BUILD_TOOLS.contains(executable) {
            RecognitionResult::Recognized(Ok(Semantic::BuildCommand))
        } else {
            RecognitionResult::NotRecognized
        }
    }
}

lazy_static! {
    static ref BUILD_TOOLS: HashSet<&'static Path> = {
		let files_paths = [
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
    fn test_make_is_recognized() {
        let input = Execution {
            executable: PathBuf::from("/usr/bin/make"),
            arguments: vec_of_strings!["make", "-C", "/home/user/build"],
            working_dir: PathBuf::from("/home/user"),
            environment: HashMap::new(),
        };

        assert_eq!(
            RecognitionResult::Recognized(Ok(Semantic::BuildCommand)),
            SUT.recognize(&input)
        )
    }

    lazy_static! {
        static ref SUT: Build = Build {};
    }
}
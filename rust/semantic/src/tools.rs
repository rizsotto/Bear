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

use std::path::PathBuf;

use crate::configuration::Compilation;
use crate::execution::Execution;
use crate::tools::build::Build;
use crate::tools::configured::Configured;
use crate::tools::RecognitionResult::{NotRecognized, Recognized};
use crate::tools::unix::Unix;
use crate::tools::wrapper::Wrapper;

mod configured;
mod wrapper;
mod matchers;
mod unix;
mod build;
mod gcc;

/// This abstraction is representing a tool which is known by us.
pub(crate) trait Tool: Send {
    /// A tool has a potential to recognize a command execution and identify
    /// the semantic of that command.
    fn recognize(&self, _: &Execution) -> RecognitionResult;
}

#[derive(Debug, PartialEq)]
pub(crate) enum RecognitionResult {
    Recognized(Result<Semantic, String>),
    NotRecognized,
}

/// Represents an executed command semantic.
#[derive(Debug, PartialEq)]
pub(crate) enum Semantic {
    UnixCommand,
    BuildCommand,
    Compiler {
        compiler: PathBuf,
        working_dir: PathBuf,
        passes: Vec<CompilerPass>,
    },
}

/// Represents a compiler call.
#[derive(Debug, PartialEq)]
pub(crate) enum CompilerPass {
    Preprocess,
    Compile {
        source: PathBuf,
        output: Option<PathBuf>,
        flags: Vec<String>,
    },
}


struct Any {
    tools: Vec<Box<dyn Tool>>,
}

impl Any {
    fn new(tools: Vec<Box<dyn Tool>>) -> Box<dyn Tool> {
        Box::new(Any { tools })
    }
}

impl Tool for Any {
    /// Any of the tool recognize the semantic, will be returned as result.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        for tool in &self.tools {
            match tool.recognize(x) {
                Recognized(result) =>
                    return Recognized(result),
                _ => continue,
            }
        }
        NotRecognized
    }
}


struct ExcludeOr {
    excludes: Vec<PathBuf>,
    or: Box<dyn Tool>,
}

impl ExcludeOr {
    fn new(excludes: &[PathBuf], tools: Vec<Box<dyn Tool>>) -> Box<dyn Tool> {
        Box::new(
            ExcludeOr {
                // exclude the executables are explicitly mentioned in the config file.
                excludes: Vec::from(excludes),
                or: Any::new(tools),
            }
        )
    }
}

impl Tool for ExcludeOr {
    /// Check if the executable is on the exclude list, return as not recognized.
    /// Otherwise delegate the recognition to the tool given.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        for exclude in &self.excludes {
            if &x.executable == exclude {
                return NotRecognized;
            }
        }
        self.or.recognize(x)
    }
}

impl From<&Compilation> for Box<dyn Tool> {
    fn from(value: &Compilation) -> Self {
        // Build the list of known compilers we will recognize by default.
        let mut tools = vec![
            Unix::new(),
            Build::new(),
            Wrapper::new(),
        ];

        // The hinted tools should be the first to recognize.
        if !value.compilers_to_recognize.is_empty() {
            let configured = Configured::from(&value.compilers_to_recognize);
            tools.insert(0, configured)
        }
        // Excluded compiler check should be done before anything.
        if value.compilers_to_exclude.is_empty() {
            Any::new(tools)
        } else {
            ExcludeOr::new(&value.compilers_to_exclude, tools)
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::vec_of_pathbuf;

    use super::*;

    #[test]
    fn test_any_when_no_match() {
        let sut = Any {
            tools: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::NotRecognize),
            ]
        };

        let input = any_execution();

        match sut.recognize(&input) {
            NotRecognized => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_any_when_match() {
        let sut = Any {
            tools: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::Recognize),
                Box::new(MockTool::NotRecognize),
            ]
        };

        let input = any_execution();

        match sut.recognize(&input) {
            Recognized(Ok(_)) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_any_when_match_fails() {
        let sut = Any {
            tools: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::RecognizeFailed),
                Box::new(MockTool::Recognize),
                Box::new(MockTool::NotRecognize),
            ]
        };

        let input = any_execution();

        match sut.recognize(&input) {
            Recognized(Err(_)) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_exclude_when_match() {
        let sut = ExcludeOr {
            excludes: vec_of_pathbuf!["/usr/bin/something"],
            or: Box::new(MockTool::Recognize),
        };

        let input = Execution {
            executable: PathBuf::from("/usr/bin/something"),
            arguments: vec![],
            working_dir: PathBuf::new(),
            environment: HashMap::new(),
        };

        match sut.recognize(&input) {
            NotRecognized => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_exclude_when_no_match() {
        let sut = ExcludeOr {
            excludes: vec_of_pathbuf!["/usr/bin/something"],
            or: Box::new(MockTool::Recognize),
        };

        let input = any_execution();

        match sut.recognize(&input) {
            Recognized(Ok(_)) => assert!(true),
            _ => assert!(false)
        }
    }

    enum MockTool {
        Recognize,
        RecognizeFailed,
        NotRecognize,
    }

    impl Tool for MockTool {
        fn recognize(&self, _: &Execution) -> RecognitionResult {
            match self {
                MockTool::Recognize =>
                    Recognized(Ok(Semantic::UnixCommand)),
                MockTool::RecognizeFailed =>
                    Recognized(Err(String::from("problem"))),
                MockTool::NotRecognize =>
                    NotRecognized,
            }
        }
    }

    fn any_execution() -> Execution {
        Execution {
            executable: PathBuf::new(),
            arguments: vec![],
            working_dir: PathBuf::new(),
            environment: HashMap::new(),
        }
    }
}

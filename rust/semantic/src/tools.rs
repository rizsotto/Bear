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

use super::tools::configured::Configured;
use super::tools::ignore::{IgnoreByArgs, IgnoreByPath};
use super::tools::unix::Unix;
use super::tools::wrapper::Wrapper;
use super::{Meaning, RecognitionResult, Tool};
use intercept::ipc::Execution;

mod configured;
mod gcc;
mod ignore;
mod matchers;
mod unix;
mod wrapper;

pub struct Builder {
    tools: Vec<Box<dyn Tool>>,
}

// TODO: write unit test for this!!!
impl Builder {
    pub fn new() -> Self {
        Builder {
            tools: vec![Unix::new(), Wrapper::new()],
        }
    }

    pub fn build(self) -> impl Tool {
        Any::new(self.tools)
    }

    pub fn compilers_to_recognize(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add the new compilers at the end of the tools.
            for compiler in compilers {
                let tool = Configured::new(compiler);
                self.tools.push(tool);
            }
        }
        self
    }

    pub fn compilers_to_exclude(mut self, compilers: &[PathBuf]) -> Self {
        if !compilers.is_empty() {
            // Add these new compilers at the front of the tools.
            let tool = IgnoreByPath::new(compilers);
            self.tools.insert(0, tool);
        }
        self
    }

    pub fn compilers_to_exclude_by_arguments(mut self, args: &[String]) -> Self {
        if !args.is_empty() {
            // Add these new compilers at the front of the tools.
            let tool = IgnoreByArgs::new(args);
            self.tools.insert(0, tool);
        }
        self
    }
}

struct Any {
    tools: Vec<Box<dyn Tool>>,
}

impl Any {
    fn new(tools: Vec<Box<dyn Tool>>) -> impl Tool {
        Any { tools }
    }
}

impl Tool for Any {
    /// Any of the tool recognize the semantic, will be returned as result.
    fn recognize(&self, x: &Execution) -> RecognitionResult {
        for tool in &self.tools {
            match tool.recognize(x) {
                RecognitionResult::Recognized(result) => {
                    return RecognitionResult::Recognized(result)
                }
                _ => continue,
            }
        }
        RecognitionResult::NotRecognized
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
            ],
        };

        let input = any_execution();

        match sut.recognize(&input) {
            RecognitionResult::NotRecognized => assert!(true),
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
            ],
        };

        let input = any_execution();

        match sut.recognize(&input) {
            RecognitionResult::Recognized(Ok(_)) => assert!(true),
            _ => assert!(false),
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
            ],
        };

        let input = any_execution();

        match sut.recognize(&input) {
            RecognitionResult::Recognized(Err(_)) => assert!(true),
            _ => assert!(false),
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
                MockTool::Recognize => RecognitionResult::Recognized(Ok(Meaning::Ignored)),
                MockTool::RecognizeFailed => {
                    RecognitionResult::Recognized(Err(String::from("problem")))
                }
                MockTool::NotRecognize => RecognitionResult::NotRecognized,
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

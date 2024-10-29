// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::{Meaning, Recognition, Tool};
use intercept::Execution;

/// Represents a set of tools, where any of them can recognize the semantic.
/// The evaluation is done in the order of the tools. The first one which
/// recognizes the semantic will be returned as result.
pub(super) struct Any {
    tools: Vec<Box<dyn Tool>>,
}

impl Any {
    pub(super) fn new(tools: Vec<Box<dyn Tool>>) -> impl Tool {
        Any { tools }
    }
}

impl Tool for Any {
    fn recognize(&self, x: &Execution) -> Recognition<Meaning> {
        for tool in &self.tools {
            match tool.recognize(x) {
                Recognition::Unknown => continue,
                result => return result,
            }
        }
        Recognition::Unknown
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::super::Meaning;
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
            Recognition::Unknown => assert!(true),
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
            Recognition::Success(_) => assert!(true),
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
            Recognition::Error(_) => assert!(true),
            _ => assert!(false),
        }
    }

    enum MockTool {
        Recognize,
        RecognizeFailed,
        NotRecognize,
    }

    impl Tool for MockTool {
        fn recognize(&self, _: &Execution) -> Recognition<Meaning> {
            match self {
                MockTool::Recognize => Recognition::Success(Meaning::Ignored),
                MockTool::RecognizeFailed => Recognition::Error(String::from("problem")),
                MockTool::NotRecognize => Recognition::Unknown,
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

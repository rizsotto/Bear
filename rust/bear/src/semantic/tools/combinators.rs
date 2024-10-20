// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::{RecognitionResult, Tool};
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

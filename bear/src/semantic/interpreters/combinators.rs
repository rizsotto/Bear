// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::{Command, Execution, Interpreter};

/// Represents a set of interpreters, where any of them can recognize the semantic.
/// The evaluation is done in the order of the interpreters. The first one which
/// recognizes the semantic will be returned as result.
pub(super) struct Any {
    interpreters: Vec<Box<dyn Interpreter>>,
}

impl Any {
    pub(super) fn new(tools: Vec<Box<dyn Interpreter>>) -> Self {
        Self {
            interpreters: tools,
        }
    }
}

impl Interpreter for Any {
    fn recognize(&self, x: &Execution) -> Option<Box<dyn Command>> {
        for tool in &self.interpreters {
            match tool.recognize(x) {
                None => continue,
                result => return result,
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::super::interpreters::generic::CompilerCall;
    use super::*;

    #[test]
    fn test_any_when_no_match() {
        let sut = Any {
            interpreters: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::NotRecognize),
            ],
        };

        let input = execution_fixture();

        assert!(
            matches!(sut.recognize(&input), None),
            "Expected None, but got a match"
        );
    }

    #[test]
    fn test_any_when_success() {
        let sut = Any {
            interpreters: vec![
                Box::new(MockTool::NotRecognize),
                Box::new(MockTool::Recognize),
                Box::new(MockTool::NotRecognize),
            ],
        };

        let input = execution_fixture();

        assert!(
            matches!(sut.recognize(&input), Some(_)),
            "Expected Some(_), got a match"
        );
    }

    enum MockTool {
        Recognize,
        NotRecognize,
    }

    impl Interpreter for MockTool {
        fn recognize(&self, _: &Execution) -> Option<Box<dyn Command>> {
            match self {
                MockTool::Recognize => Some(command_fixture()),
                MockTool::NotRecognize => None,
            }
        }
    }

    fn execution_fixture() -> Execution {
        Execution {
            executable: PathBuf::new(),
            arguments: vec![],
            working_dir: PathBuf::new(),
            environment: HashMap::new(),
        }
    }

    fn command_fixture() -> Box<dyn Command> {
        Box::new(CompilerCall {
            compiler: PathBuf::new(),
            working_dir: PathBuf::new(),
            passes: vec![],
        })
    }
}

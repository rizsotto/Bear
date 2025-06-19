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
    fn recognize(&self, x: &Execution) -> Option<Command> {
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
    use super::*;
    use crate::semantic::interpreters::CompilerCommand;
    use std::collections::HashMap;
    use std::path::PathBuf;

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
        fn recognize(&self, _: &Execution) -> Option<Command> {
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

    fn command_fixture() -> Command {
        Command::Compiler(CompilerCommand {})
        // CompilerCall {
        //     compiler: PathBuf::new(),
        //     working_dir: PathBuf::new(),
        //     passes: vec![],
        // }
    }
}

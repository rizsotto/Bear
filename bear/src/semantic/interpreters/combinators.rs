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
    use crate::semantic::command::CompilerCommand;
    use crate::semantic::MockInterpreter;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_any_when_no_match() {
        let mut mock1 = MockInterpreter::new();
        let mut mock2 = MockInterpreter::new();
        let mut mock3 = MockInterpreter::new();

        // Set up all mocks to return None (not recognize)
        mock1.expect_recognize().returning(|_| None);
        mock2.expect_recognize().returning(|_| None);
        mock3.expect_recognize().returning(|_| None);

        let sut = Any {
            interpreters: vec![Box::new(mock1), Box::new(mock2), Box::new(mock3)],
        };

        let input = execution_fixture();

        assert!(
            sut.recognize(&input).is_none(),
            "Expected None, but got a match"
        );
    }

    #[test]
    fn test_any_when_success() {
        let mut mock1 = MockInterpreter::new();
        let mut mock2 = MockInterpreter::new();
        let mock3 = MockInterpreter::new();

        // First mock returns None, second returns Some, third should not be called
        mock1.expect_recognize().returning(|_| None);
        mock2
            .expect_recognize()
            .returning(|_| Some(command_fixture()));
        // mock3 should not be called since mock2 returns a match

        let sut = Any {
            interpreters: vec![Box::new(mock1), Box::new(mock2), Box::new(mock3)],
        };

        let input = execution_fixture();

        assert!(
            sut.recognize(&input).is_some(),
            "Expected Some(_), got a match"
        );
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
        Command::Compiler(CompilerCommand::new(PathBuf::new(), PathBuf::new(), vec![]))
    }
}

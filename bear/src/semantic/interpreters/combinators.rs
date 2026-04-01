// SPDX-License-Identifier: GPL-3.0-or-later

use super::super::{Execution, Interpreter, RecognizeResult};

/// Represents a set of interpreters, where any of them can recognize the semantic.
/// The evaluation is done in the order of the interpreters. The first one which
/// recognizes the semantic will be returned as result.
pub(super) struct Any {
    interpreters: Vec<Box<dyn Interpreter>>,
}

impl Any {
    pub(super) fn new(tools: Vec<Box<dyn Interpreter>>) -> Self {
        Self { interpreters: tools }
    }
}

impl Interpreter for Any {
    fn recognize(&self, mut execution: Execution) -> RecognizeResult {
        for tool in &self.interpreters {
            match tool.recognize(execution) {
                RecognizeResult::NotRecognized(returned) => execution = returned,
                result => return result,
            }
        }
        RecognizeResult::NotRecognized(execution)
    }
}

/// A combinator that logs the input execution before delegating to the inner interpreter.
pub(super) struct InputLogger<T: Interpreter> {
    inner: T,
}

impl<T: Interpreter> InputLogger<T> {
    pub(super) fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Interpreter> Interpreter for InputLogger<T> {
    fn recognize(&self, execution: Execution) -> RecognizeResult {
        log::debug!("Recognizing execution: {execution:?}");
        self.inner.recognize(execution)
    }
}

/// A combinator that logs the output result after delegating to the inner interpreter.
pub(super) struct OutputLogger<T: Interpreter> {
    inner: T,
    name: String,
}

impl<T: Interpreter> OutputLogger<T> {
    pub(super) fn new(inner: T, name: impl Into<String>) -> Self {
        Self { inner, name: name.into() }
    }
}

impl<T: Interpreter> Interpreter for OutputLogger<T> {
    fn recognize(&self, execution: Execution) -> RecognizeResult {
        let result = self.inner.recognize(execution);
        log::debug!("{:20}: {result:?}", self.name);
        result
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::semantic::CompilerCommand;
    use crate::semantic::MockInterpreter;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_any_when_no_match() {
        let mut mock1 = MockInterpreter::new();
        let mut mock2 = MockInterpreter::new();
        let mut mock3 = MockInterpreter::new();

        mock1.expect_recognize().returning(RecognizeResult::NotRecognized);
        mock2.expect_recognize().returning(RecognizeResult::NotRecognized);
        mock3.expect_recognize().returning(RecognizeResult::NotRecognized);

        let sut = Any { interpreters: vec![Box::new(mock1), Box::new(mock2), Box::new(mock3)] };

        let input = execution_fixture();
        assert!(matches!(sut.recognize(input), RecognizeResult::NotRecognized(_)), "Expected NotRecognized");
    }

    #[test]
    fn test_any_when_success() {
        let mut mock1 = MockInterpreter::new();
        let mut mock2 = MockInterpreter::new();
        let mock3 = MockInterpreter::new();

        mock1.expect_recognize().returning(RecognizeResult::NotRecognized);
        mock2.expect_recognize().returning(|_| command_fixture());
        // mock3 should not be called since mock2 returns a match

        let sut = Any { interpreters: vec![Box::new(mock1), Box::new(mock2), Box::new(mock3)] };

        let input = execution_fixture();
        assert!(matches!(sut.recognize(input), RecognizeResult::Recognized(_)), "Expected Recognized");
    }

    fn execution_fixture() -> Execution {
        Execution {
            executable: PathBuf::from("/usr/bin/ls"),
            arguments: vec!["ls".to_string()],
            working_dir: PathBuf::new(),
            environment: HashMap::new(),
        }
    }

    fn command_fixture() -> RecognizeResult {
        RecognizeResult::Recognized(CompilerCommand::new(PathBuf::new(), PathBuf::new(), vec![]))
    }

    #[test]
    fn test_input_logger_passes_through_results() {
        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "main.c"],
            "/home/user",
            HashMap::new(),
        );

        struct AlwaysNotRecognized;
        impl Interpreter for AlwaysNotRecognized {
            fn recognize(&self, execution: Execution) -> RecognizeResult {
                RecognizeResult::NotRecognized(execution)
            }
        }

        let logger = InputLogger::new(AlwaysNotRecognized);
        assert!(matches!(logger.recognize(execution), RecognizeResult::NotRecognized(_)));
    }

    #[test]
    fn test_output_logger_passes_through_results() {
        let execution = Execution::from_strings(
            "/usr/bin/gcc",
            vec!["gcc", "-c", "main.c"],
            "/home/user",
            HashMap::new(),
        );

        struct AlwaysNotRecognized;
        impl Interpreter for AlwaysNotRecognized {
            fn recognize(&self, execution: Execution) -> RecognizeResult {
                RecognizeResult::NotRecognized(execution)
            }
        }

        let logger = OutputLogger::new(AlwaysNotRecognized, "test");
        assert!(matches!(logger.recognize(execution), RecognizeResult::NotRecognized(_)));
    }
}

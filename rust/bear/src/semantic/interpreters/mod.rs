// SPDX-License-Identifier: GPL-3.0-or-later

use super::interpreters::combinators::Any;
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::Interpreter;
use crate::config;
use std::path::PathBuf;

mod combinators;
mod generic;
mod ignore;
mod matchers;

/// Creates an interpreter to recognize the compiler calls.
///
/// Using the configuration we can define which compilers to include and exclude.
/// Also read the environment variables to detect the compiler to include (and
/// make sure those are not excluded either).
// TODO: Use the CC or CXX environment variables to detect the compiler to include.
//       Use the CC or CXX environment variables and make sure those are not excluded.
//       Make sure the environment variables are passed to the method.
// TODO: Take environment variables as input.
pub fn create_interpreter<'a>(config: &config::Main) -> impl Interpreter + 'a {
    let compilers_to_include = match &config.intercept {
        config::Intercept::Wrapper { executables, .. } => executables.clone(),
        _ => vec![],
    };
    let compilers_to_exclude = match &config.output {
        config::Output::Clang { compilers, .. } => compilers
            .iter()
            .filter(|compiler| compiler.ignore == config::IgnoreOrConsider::Always)
            .map(|compiler| compiler.path.clone())
            .collect(),
        _ => vec![],
    };

    let mut interpreters: Vec<Box<dyn Interpreter>> = vec![
        // ignore executables which are not compilers,
        Box::new(IgnoreByPath::default()),
        // recognize default compiler
        Box::new(Generic::from(&[PathBuf::from("/usr/bin/cc")])),
    ];

    if !compilers_to_include.is_empty() {
        let tool = Generic::from(&compilers_to_include);
        interpreters.push(Box::new(tool));
    }

    if !compilers_to_exclude.is_empty() {
        let tool = IgnoreByPath::from(&compilers_to_exclude);
        interpreters.insert(0, Box::new(tool));
    }

    Any::new(interpreters)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::{CompilerCall, Execution, Recognition};
    use super::*;
    use crate::config;
    use crate::config::{DuplicateFilter, Format, SourceFilter};
    use crate::{vec_of_pathbuf, vec_of_strings};

    fn any_execution() -> Execution {
        Execution {
            executable: PathBuf::from("/usr/bin/cc"),
            arguments: vec_of_strings!["cc", "-c", "-Wall", "main.c"],
            environment: HashMap::new(),
            working_dir: PathBuf::from("/home/user"),
        }
    }

    #[test]
    fn test_create_interpreter_with_default_config() {
        let config = config::Main::default();

        let interpreter = create_interpreter(&config);
        let input = any_execution();

        match interpreter.recognize(&input) {
            Recognition::Success(CompilerCall { .. }) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_create_interpreter_with_compilers_to_include() {
        let config = config::Main {
            intercept: config::Intercept::Wrapper {
                executables: vec_of_pathbuf!["/usr/bin/cc"],
                path: PathBuf::from("/usr/libexec/bear"),
                directory: PathBuf::from("/tmp"),
            },
            ..Default::default()
        };

        let interpreter = create_interpreter(&config);
        let input = any_execution();

        match interpreter.recognize(&input) {
            Recognition::Success(CompilerCall { .. }) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_create_interpreter_with_compilers_to_exclude() {
        let config = config::Main {
            output: config::Output::Clang {
                compilers: vec![config::Compiler {
                    path: PathBuf::from("/usr/bin/cc"),
                    ignore: config::IgnoreOrConsider::Always,
                    arguments: config::Arguments::default(),
                }],
                sources: SourceFilter::default(),
                duplicates: DuplicateFilter::default(),
                format: Format::default(),
            },
            ..Default::default()
        };

        let interpreter = create_interpreter(&config);
        let input = any_execution();

        let result = interpreter.recognize(&input);

        assert_eq!(
            result,
            Recognition::Ignored("compiler specified in config to ignore".into())
        );
    }
}

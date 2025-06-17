// SPDX-License-Identifier: GPL-3.0-or-later

use super::interpreters::combinators::Any;
use super::interpreters::generic::Generic;
use super::interpreters::ignore::IgnoreByPath;
use super::Interpreter;
use crate::config;
use std::path::PathBuf;

mod combinators;
pub(crate) mod generic;
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
pub fn create<'a>(config: &config::Main) -> impl Interpreter + 'a {
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

    use super::*;
    use crate::config;
    use crate::intercept::{execution, Execution};
    use crate::semantic::Recognition;

    #[test]
    fn test_create_interpreter_with_default_config() {
        let config = config::Main::default();

        let interpreter = create(&config);

        let result = interpreter.recognize(&EXECUTION);
        assert!(matches!(result, Recognition::Success(_)));
    }

    #[test]
    fn test_create_interpreter_with_compilers_to_include() {
        let config = config::Main {
            intercept: config::Intercept::Wrapper {
                executables: vec!["/usr/bin/cc".into()],
                path: "/usr/libexec/bear".into(),
                directory: "/tmp".into(),
            },
            ..Default::default()
        };

        let interpreter = create(&config);

        let result = interpreter.recognize(&EXECUTION);
        assert!(matches!(result, Recognition::Success(_)));
    }

    // #[test]
    // fn test_create_interpreter_with_compilers_to_exclude() {
    //     let config = config::Main {
    //         output: config::Output::Clang {
    //             compilers: vec![config::Compiler {
    //                 path: PathBuf::from("/usr/bin/cc"),
    //                 ignore: config::IgnoreOrConsider::Always,
    //                 arguments: config::Arguments::default(),
    //             }],
    //             sources: config::SourceFilter::default(),
    //             duplicates: config::DuplicateFilter::default(),
    //             format: config::Format::default(),
    //         },
    //         ..Default::default()
    //     };
    //
    //     let interpreter = create(&config);
    //
    //     let result = interpreter.recognize(&EXECUTION);
    //     assert!(matches!(result, Recognition::Ignored(_)));
    // }

    static EXECUTION: std::sync::LazyLock<Execution> = std::sync::LazyLock::new(|| {
        execution(
            "/usr/bin/cc",
            vec!["cc", "-c", "-Wall", "main.c"],
            "/home/user",
            HashMap::new(),
        )
    });
}

// SPDX-License-Identifier: GPL-3.0-or-later

//! Cray Fortran compiler interpreter.
//!
//! This module provides flag interpretation for Cray Fortran compilers (crayftn).
//! Cray Fortran compilers have their own specific flags and behaviors, with some
//! compatibility with GCC-style flags but also Cray-specific extensions.

use crate::semantic::interpreters::compilers::gcc::parse_arguments_and_environment;
use crate::semantic::interpreters::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use crate::semantic::{ArgumentKind, Command, CompilerPass, Interpreter, PassEffect};
// Cray Fortran flag definitions. Generated at build time from flags/cray_fortran.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_cray_fortran.rs"));

/// Cray Fortran compiler interpreter.
///
/// This interpreter handles Cray Fortran compilers (crayftn) which have their own
/// specific flags and behaviors. Cray compilers support some GCC-compatible flags
/// but also have Cray-specific extensions for HPC environments.
pub struct CrayFortranInterpreter {
    analyzer: FlagAnalyzer,
}

impl Default for CrayFortranInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl CrayFortranInterpreter {
    /// Create a new Cray Fortran interpreter.
    pub fn new() -> Self {
        Self { analyzer: FlagAnalyzer::new(&CRAY_FORTRAN_FLAGS) }
    }
}

impl Interpreter for CrayFortranInterpreter {
    fn recognize(&self, execution: &crate::intercept::Execution) -> Option<Command> {
        use crate::semantic::CompilerCommand;

        // Parse arguments using Cray Fortran-specific flag definitions
        let parsed = parse_arguments_and_environment(&self.analyzer, execution);

        Some(Command::Compiler(CompilerCommand::new(
            execution.working_dir.clone(),
            execution.executable.clone(),
            parsed,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intercept::Execution;
    use std::collections::HashMap;

    fn create_execution(executable: &str, args: Vec<&str>, working_dir: &str) -> Execution {
        Execution::from_strings(executable, args, working_dir, HashMap::new())
    }

    #[test]
    fn test_cray_fortran_basic_compilation() {
        let interpreter = CrayFortranInterpreter::new();
        let execution = create_execution("crayftn", vec!["crayftn", "-c", "test.f90"], "/project");

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 3);

            // Check -c flag
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling))
            );
        }
    }

    #[test]
    fn test_cray_fortran_preprocessing_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution =
            create_execution("crayftn", vec!["crayftn", "-DDEBUG", "-I/usr/include", "test.f90"], "/project");

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check -D flag (preprocessing)
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            // Check -I flag (preprocessing)
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
        }
    }

    #[test]
    fn test_cray_fortran_linking_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution =
            create_execution("crayftn", vec!["crayftn", "-add-rpath", "-lm", "test.o"], "/project");

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check -add-rpath flag (linking)
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
            // Check -l flag (linking)
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking))
            );
        }
    }

    #[test]
    fn test_cray_fortran_cray_specific_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution = create_execution(
            "crayftn",
            vec!["crayftn", "-craylibs", "-target-cpu=x86_64", "test.f90"],
            "/project",
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 4);
            // Should recognize Cray-specific flags
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(PassEffect::None));
            assert_eq!(parsed.arguments[2].kind(), ArgumentKind::Other(PassEffect::None));
        }
    }

    #[test]
    fn test_cray_fortran_openmp_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution = create_execution("crayftn", vec!["crayftn", "-openmp", "test.f90"], "/project");

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 3);
            // Check -openmp flag
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(PassEffect::None));
        }
    }

    #[test]
    fn test_flag_table_invariants() {
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&CRAY_FORTRAN_FLAGS);
    }
}

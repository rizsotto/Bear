// SPDX-License-Identifier: GPL-3.0-or-later

//! Intel Fortran compiler interpreter.
//!
//! This module provides flag interpretation for Intel Fortran compilers (ifort, ifx).
//! Intel Fortran compilers have their own specific flags and behaviors, though they
//! share some compatibility with GCC-style flags.

use crate::semantic::interpreters::compilers::gcc::parse_arguments_and_environment;
use crate::semantic::interpreters::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use crate::semantic::{ArgumentKind, Command, CompilerPass, Interpreter, PassEffect};
// Intel Fortran flag definitions. Generated at build time from flags/intel_fortran.yaml.
include!(concat!(env!("OUT_DIR"), "/flags_intel_fortran.rs"));

/// Intel Fortran compiler interpreter.
///
/// This interpreter handles Intel Fortran compilers (ifort, ifx) which have their own
/// specific flags and behaviors. Intel Fortran compilers support many GCC-compatible flags
/// but also have Intel-specific extensions.
pub struct IntelFortranInterpreter {
    analyzer: FlagAnalyzer,
}

impl Default for IntelFortranInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelFortranInterpreter {
    /// Create a new Intel Fortran interpreter.
    pub fn new() -> Self {
        Self { analyzer: FlagAnalyzer::new(&INTEL_FORTRAN_FLAGS) }
    }
}

impl Interpreter for IntelFortranInterpreter {
    fn recognize(&self, execution: &crate::intercept::Execution) -> Option<Command> {
        use crate::semantic::CompilerCommand;

        // Parse arguments using Intel Fortran-specific flag definitions
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
    fn test_intel_fortran_basic_compilation() {
        let interpreter = IntelFortranInterpreter::new();
        let execution = create_execution("ifort", vec!["ifort", "-c", "test.f90"], "/project");

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
    fn test_intel_fortran_preprocessing_flags() {
        let interpreter = IntelFortranInterpreter::new();
        let execution = create_execution(
            "ifort",
            vec!["ifort", "-fpp", "-DDEBUG", "-I/usr/include", "test.f90"],
            "/project",
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check -fpp flag (preprocessing)
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
            // Check -D flag (preprocessing)
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing))
            );
        }
    }

    #[test]
    fn test_intel_fortran_linking_flags() {
        let interpreter = IntelFortranInterpreter::new();
        let execution =
            create_execution("ifort", vec!["ifort", "-shared-intel", "-lm", "test.o"], "/project");

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check -shared-intel flag (linking)
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
    fn test_intel_fortran_info_flags() {
        let interpreter = IntelFortranInterpreter::new();
        let execution = create_execution("ifort", vec!["ifort", "--version"], "/project");

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check --version flag (info)
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(PassEffect::InfoAndExit));
        }
    }

    #[test]
    fn test_flag_table_invariants() {
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&INTEL_FORTRAN_FLAGS);
    }
}

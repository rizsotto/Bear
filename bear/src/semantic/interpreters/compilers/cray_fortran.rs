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

    #[test]
    fn test_flag_table_invariants() {
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&CRAY_FORTRAN_FLAGS);
    }
}

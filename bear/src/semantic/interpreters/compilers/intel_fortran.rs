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

    #[test]
    fn test_flag_table_invariants() {
        crate::semantic::interpreters::compilers::gcc::assert_flag_table_invariants(&INTEL_FORTRAN_FLAGS);
    }
}

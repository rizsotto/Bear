// SPDX-License-Identifier: GPL-3.0-or-later

//! Cray Fortran compiler interpreter.
//!
//! This module provides flag interpretation for Cray Fortran compilers (crayftn).
//! Cray Fortran compilers have their own specific flags and behaviors, with some
//! compatibility with GCC-style flags but also Cray-specific extensions.

use crate::semantic::interpreters::compilers::gcc::parse_arguments_and_environment;
use crate::semantic::interpreters::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use crate::semantic::{ArgumentKind, Command, CompilerPass, Interpreter};
use std::sync::LazyLock;

/// Flag definitions for Cray Fortran compilers
///
/// Based on Cray Fortran Reference Manual and HPE Cray Programming Environment User Guide:
/// https://support.hpe.com/hpesc/public/docDisplay?docId=a00115296en_us&page=Fortran_Command-line_Options.html
static CRAY_FORTRAN_FLAGS: LazyLock<Vec<FlagRule>> = LazyLock::new(|| {
    let mut flags = vec![
        FlagRule::new(
            FlagPattern::Exactly("-no-add-rpath-shared", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--no-custom-ld-script", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-no-add-runpath", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-add-rpath-shared", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--no-as-needed", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-no-gcc-rpath", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-no-add-rpath", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-add-runpath", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-no-as-needed", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("--as-needed", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-gcc-rpath", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-add-rpath", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-as-needed", 0),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-qno-openmp", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-noopenmp", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Mnoopenmp", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-default64", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-openmp", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-dynamic", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-shared", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static", 0),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(FlagPattern::Exactly("-VVV", 0), ArgumentKind::Other(None)),
        FlagRule::new(FlagPattern::Exactly("-mp", 0), ArgumentKind::Other(None)),
        FlagRule::new(FlagPattern::Exactly("-VV", 0), ArgumentKind::Other(None)),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("--custom-ld-script="),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-target-network="),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-target-accel="),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-target-cpu="),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-A"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-b"),
            ArgumentKind::Output,
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-D"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-d"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-e"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-f"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-G"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-h"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-I"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-J"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-K"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-l"),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-L"),
            ArgumentKind::Other(Some(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-m"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-M"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-N"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-O"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-o"),
            ArgumentKind::Output,
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-p"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-Q"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-r"),
            ArgumentKind::Other(Some(CompilerPass::Info)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-R"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-s"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-U"),
            ArgumentKind::Other(Some(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-W"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-x"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-Y"),
            ArgumentKind::Other(None),
        ),
        FlagRule::new(FlagPattern::Prefix("--cray", 0), ArgumentKind::Other(None)),
        FlagRule::new(FlagPattern::Prefix("-cray", 0), ArgumentKind::Other(None)),
        FlagRule::new(
            FlagPattern::Exactly("-c", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-E", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(FlagPattern::Exactly("-F", 0), ArgumentKind::Other(None)),
        FlagRule::new(FlagPattern::Exactly("-g", 0), ArgumentKind::Other(None)),
        FlagRule::new(
            FlagPattern::Exactly("-S", 0),
            ArgumentKind::Other(Some(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-T", 0),
            ArgumentKind::Other(Some(CompilerPass::Info)),
        ),
        FlagRule::new(FlagPattern::Exactly("-v", 0), ArgumentKind::Other(None)),
        FlagRule::new(FlagPattern::Exactly("-V", 0), ArgumentKind::Other(None)),
    ];

    // Sort by flag length descending to ensure longer matches are tried first
    flags.sort_by(|a, b| b.pattern.flag().len().cmp(&a.pattern.flag().len()));

    flags
});

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
        Self {
            analyzer: FlagAnalyzer::new(&CRAY_FORTRAN_FLAGS),
        }
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
                ArgumentKind::Other(Some(CompilerPass::Compiling))
            );
        }
    }

    #[test]
    fn test_cray_fortran_preprocessing_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution = create_execution(
            "crayftn",
            vec!["crayftn", "-DDEBUG", "-I/usr/include", "test.f90"],
            "/project",
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check -D flag (preprocessing)
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
            // Check -I flag (preprocessing)
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Preprocessing))
            );
        }
    }

    #[test]
    fn test_cray_fortran_linking_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution = create_execution(
            "crayftn",
            vec!["crayftn", "-add-rpath", "-lm", "test.o"],
            "/project",
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            // Check -add-rpath flag (linking)
            assert_eq!(
                parsed.arguments[1].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
            );
            // Check -l flag (linking)
            assert_eq!(
                parsed.arguments[2].kind(),
                ArgumentKind::Other(Some(CompilerPass::Linking))
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
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(None));
            assert_eq!(parsed.arguments[2].kind(), ArgumentKind::Other(None));
        }
    }

    #[test]
    fn test_cray_fortran_openmp_flags() {
        let interpreter = CrayFortranInterpreter::new();
        let execution = create_execution(
            "crayftn",
            vec!["crayftn", "-openmp", "test.f90"],
            "/project",
        );

        let result = interpreter.recognize(&execution);
        assert!(result.is_some());

        if let Some(Command::Compiler(parsed)) = result {
            assert_eq!(parsed.arguments.len(), 3);
            // Check -openmp flag
            assert_eq!(parsed.arguments[1].kind(), ArgumentKind::Other(None));
        }
    }
}

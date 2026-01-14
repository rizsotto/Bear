// SPDX-License-Identifier: GPL-3.0-or-later

//! Intel Fortran compiler interpreter.
//!
//! This module provides flag interpretation for Intel Fortran compilers (ifort, ifx).
//! Intel Fortran compilers have their own specific flags and behaviors, though they
//! share some compatibility with GCC-style flags.

use crate::semantic::interpreters::compilers::gcc::parse_arguments_and_environment;
use crate::semantic::interpreters::matchers::{FlagAnalyzer, FlagPattern, FlagRule};
use crate::semantic::{ArgumentKind, Command, CompilerPass, Interpreter, PassEffect};
use std::sync::LazyLock;

/// Flag definitions for Intel Fortran compilers
///
/// Based on Intel® Fortran Compiler Developer Guide and Reference:
/// https://www.intel.com/content/www/us/en/developer/tools/oneapi/fortran-compiler.html
static INTEL_FORTRAN_FLAGS: LazyLock<Vec<FlagRule>> = LazyLock::new(|| {
    let mut flags = vec![
        FlagRule::new(
            FlagPattern::Exactly("-preprocess-only", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-debug-parameters", 1),
            ArgumentKind::Other(PassEffect::InfoAndExit),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-shared-intel", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static-intel", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static-libgcc", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(FlagPattern::Exactly("-nogen-interfaces", 0), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(
            FlagPattern::Exactly("shared-libgcc", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(FlagPattern::Exactly("-gen-interfaces", 1), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(
            FlagPattern::Exactly("-nostartfiles", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-nodefaultlibs", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-gen-dep", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(FlagPattern::Exactly("-dumpmachine", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(FlagPattern::Exactly("--version", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(
            FlagPattern::Exactly("-nostdlib", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-pthread", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(FlagPattern::Exactly("-dryrun", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(
            FlagPattern::Exactly("-shared", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-static", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-isystem", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-isysroot", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-include", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-iquote", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Xlinker", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(FlagPattern::Exactly("-debug", 1), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(
            FlagPattern::Exactly("-nofpp", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-undef", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(FlagPattern::Exactly("--help", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(
            FlagPattern::Exactly("-MMD", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-fpp", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-Ep", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MF", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-MD", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-T", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-C", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-u", 1),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(FlagPattern::Exactly("-V", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(
            FlagPattern::Exactly("-X", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-c", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-E", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(FlagPattern::Exactly("-o", 1), ArgumentKind::Output),
        FlagRule::new(
            FlagPattern::Exactly("-P", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-r", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-S", 0),
            ArgumentKind::Other(PassEffect::StopsAt(CompilerPass::Assembling)),
        ),
        FlagRule::new(
            FlagPattern::Exactly("-s", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-Xoption,link", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-Xoption,fpp", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-Xoption,cpp", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(FlagPattern::Prefix("-Xoption,asm", 0), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(
            FlagPattern::ExactlyWithEq("--sysroot"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-include"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-D"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-I"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-L"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-l"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithGluedOrSep("-U"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(
            FlagPattern::ExactlyWithEq("-std"),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(FlagPattern::Prefix("-diag-", 0), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(FlagPattern::Prefix("--help", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(FlagPattern::Prefix("-FA", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(FlagPattern::Prefix("-Fa", 0), ArgumentKind::Other(PassEffect::InfoAndExit)),
        FlagRule::new(
            FlagPattern::Prefix("-Wl", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Linking)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-Wp", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Preprocessing)),
        ),
        FlagRule::new(FlagPattern::Prefix("--", 0), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(
            FlagPattern::Prefix("-f", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-g", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("-m", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(FlagPattern::Prefix("-no", 0), ArgumentKind::Other(PassEffect::None)),
        FlagRule::new(
            FlagPattern::Prefix("-O", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(FlagPattern::Prefix("-v", 0), ArgumentKind::Other(PassEffect::DriverOption)),
        FlagRule::new(
            FlagPattern::Prefix("-x", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
        FlagRule::new(
            FlagPattern::Prefix("@", 0),
            ArgumentKind::Other(PassEffect::Configures(CompilerPass::Compiling)),
        ),
    ];

    // Sort by flag length descending to ensure longer matches are tried first
    flags.sort_by(|a, b| b.pattern.flag().len().cmp(&a.pattern.flag().len()));

    flags
});

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
}

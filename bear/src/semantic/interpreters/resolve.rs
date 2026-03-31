// SPDX-License-Identifier: GPL-3.0-or-later

//! Executable path resolution for intercepted commands.
//!
//! This module provides:
//! - [`ExecutableResolver`] — resolves bare executable filenames to absolute paths
//!   using the execution's `PATH` environment variable.
//! - [`ResolveExecutable`] — an interpreter decorator that applies resolution
//!   before delegating to an inner interpreter.
//!
//! # Why resolution is needed
//!
//! In preload mode, `p`-variant exec functions (`execvp`, `execlp`, `posix_spawnp`)
//! pass bare filenames (e.g. `gcc`) which the preload shim reports as-is. This
//! decorator resolves them to absolute paths before semantic analysis, so the
//! compilation database always contains full compiler paths.

use crate::intercept::Execution;
use crate::semantic::{Command, Interpreter};
use std::path::{Path, PathBuf};

/// Resolves bare executable filenames to absolute paths.
///
/// Uses the execution's own `PATH` to search, falling back to a system
/// default path (from `confstr(_CS_PATH)`) when `PATH` is not set.
struct ExecutableResolver {
    fallback_path: String,
}

impl ExecutableResolver {
    fn new(fallback_path: String) -> Self {
        Self { fallback_path }
    }

    /// Resolves the executable path from an execution.
    ///
    /// - If the executable is already absolute, returns it unchanged.
    /// - Otherwise, searches the execution's `PATH` (or the fallback path)
    ///   using `which_in`.
    /// - If resolution fails, returns the original path unchanged.
    fn resolve(&self, execution: &Execution) -> PathBuf {
        if execution.executable.is_absolute() {
            return execution.executable.clone();
        }

        let search_path =
            execution.environment.get("PATH").map(|s| s.as_str()).unwrap_or(&self.fallback_path);

        Self::which_in(&execution.executable, search_path, &execution.working_dir)
            .unwrap_or_else(|| execution.executable.clone())
    }

    fn which_in(executable: &Path, search_path: &str, working_dir: &Path) -> Option<PathBuf> {
        which::which_in(executable, Some(search_path), working_dir).ok()
    }
}

/// Interpreter decorator that resolves bare executable filenames to absolute
/// paths before delegating to the inner interpreter.
pub struct ResolveExecutable<T: Interpreter> {
    inner: T,
    resolver: ExecutableResolver,
}

impl<T: Interpreter> ResolveExecutable<T> {
    pub fn new(inner: T, fallback_path: String) -> Self {
        Self { inner, resolver: ExecutableResolver::new(fallback_path) }
    }
}

impl<T: Interpreter> Interpreter for ResolveExecutable<T> {
    fn recognize(&self, execution: &Execution) -> Option<Command> {
        let resolved = self.resolver.resolve(execution);
        if resolved == execution.executable {
            return self.inner.recognize(execution);
        }

        let resolved_execution = Execution {
            executable: resolved,
            arguments: execution.arguments.clone(),
            working_dir: execution.working_dir.clone(),
            environment: execution.environment.clone(),
        };
        self.inner.recognize(&resolved_execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Returns a bare executable name and search path that exist on the current platform.
    fn platform_executable_and_path() -> (&'static str, String) {
        #[cfg(unix)]
        {
            ("sh", "/usr/bin:/bin".to_string())
        }
        #[cfg(windows)]
        {
            // cmd.exe lives in the Windows system directory
            let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
            let system32 = format!(r"{}\System32", system_root);
            ("cmd.exe", system32)
        }
    }

    #[test]
    fn test_resolve_absolute_path_unchanged() {
        let (_, search_path) = platform_executable_and_path();
        let resolver = ExecutableResolver::new(search_path);
        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec![],
            working_dir: PathBuf::from("/tmp"),
            environment: HashMap::new(),
        };

        let result = resolver.resolve(&execution);
        assert_eq!(result, PathBuf::from("/usr/bin/gcc"));
    }

    #[test]
    fn test_resolve_bare_name_uses_path() {
        let (exe, search_path) = platform_executable_and_path();
        let resolver = ExecutableResolver::new(String::new());
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), search_path);

        let execution = Execution {
            executable: PathBuf::from(exe),
            arguments: vec![],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            environment: env,
        };

        let result = resolver.resolve(&execution);
        assert!(result.is_absolute(), "Expected absolute path, got: {:?}", result);
        assert_eq!(result.file_name().unwrap(), exe);
    }

    #[test]
    fn test_resolve_bare_name_uses_fallback_when_no_path() {
        let (exe, search_path) = platform_executable_and_path();
        let resolver = ExecutableResolver::new(search_path);

        let execution = Execution {
            executable: PathBuf::from(exe),
            arguments: vec![],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            environment: HashMap::new(), // no PATH
        };

        let result = resolver.resolve(&execution);
        assert!(result.is_absolute(), "Expected absolute path, got: {:?}", result);
        assert_eq!(result.file_name().unwrap(), exe);
    }

    #[test]
    fn test_resolve_unknown_binary_returns_original() {
        let (_, search_path) = platform_executable_and_path();
        let resolver = ExecutableResolver::new(search_path);

        let execution = Execution {
            executable: PathBuf::from("nonexistent_compiler_xyz_12345"),
            arguments: vec![],
            working_dir: PathBuf::from("/tmp"),
            environment: HashMap::new(),
        };

        let result = resolver.resolve(&execution);
        assert_eq!(result, PathBuf::from("nonexistent_compiler_xyz_12345"));
    }

    #[test]
    fn test_decorator_resolves_before_delegating() {
        use crate::semantic::MockInterpreter;

        let (exe, search_path) = platform_executable_and_path();

        let mut mock = MockInterpreter::new();
        mock.expect_recognize().withf(|exec| exec.executable.is_absolute()).returning(|_| None);

        let decorator = ResolveExecutable::new(mock, String::new());

        let mut env = HashMap::new();
        env.insert("PATH".to_string(), search_path);

        let execution = Execution {
            executable: PathBuf::from(exe),
            arguments: vec![exe.to_string()],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            environment: env,
        };

        // The mock expects an absolute path — if the decorator works,
        // the mock's withf predicate will pass.
        let _ = decorator.recognize(&execution);
    }

    #[test]
    fn test_decorator_passes_absolute_path_unchanged() {
        use crate::semantic::MockInterpreter;

        let mut mock = MockInterpreter::new();
        mock.expect_recognize().withf(|exec| exec.executable == *"/usr/bin/gcc").returning(|_| None);

        let decorator = ResolveExecutable::new(mock, "/usr/bin:/bin".to_string());

        let execution = Execution {
            executable: PathBuf::from("/usr/bin/gcc"),
            arguments: vec!["/usr/bin/gcc".to_string()],
            working_dir: PathBuf::from("/tmp"),
            environment: HashMap::new(),
        };

        let _ = decorator.recognize(&execution);
    }
}

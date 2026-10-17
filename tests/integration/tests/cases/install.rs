// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for the installation layout that `scripts/install.sh` produces.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::InstallEnvironment;
#[cfg(all(has_preload_library, not(target_os = "macos"), has_executable_shell, has_executable_true))]
use crate::fixtures::infrastructure::{EventMatcher, TestEnvironment};
use anyhow::Result;

/// Distribution tooling (RPM's debuginfo extraction and strip step) only
/// processes shared libraries that are executable, so packagers had to
/// `chmod` the preload library after running the install script.
#[test]
fn preload_library_is_installed_executable() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let install = InstallEnvironment::new()?;

    let sut = install.install_dir().join("libexec/bear").join(INTERCEPT_LIBDIR).join(PRELOAD_LIBRARY);

    let mode = std::fs::metadata(&sut)?.permissions().mode();
    assert_eq!(mode & 0o777, 0o755, "{} has mode {:o}", sut.display(), mode & 0o777);
    Ok(())
}

/// Distributions without a `libexec` directory (Arch Linux) choose the
/// parent of the `bear/` directory. The driver finds its siblings by
/// relative path, so the relocated install still intercepts: the `true`
/// that the shell runs is only reported when the preload library loads.
#[test]
#[cfg(all(has_preload_library, not(target_os = "macos"), has_executable_shell, has_executable_true))]
fn libexecdir_relocates_the_helpers() -> Result<()> {
    let install = InstallEnvironment::with_libexecdir("lib")?;
    let prefix = install.install_dir().to_path_buf();
    let env = TestEnvironment::with_install("libexecdir_relocates_the_helpers", install)?;

    env.run_bear_success(&["intercept", "--output", "events.json", "--", SHELL_PATH, "-c", TRUE_PATH])?;
    let sut = env.load_events_file("events.json")?;

    sut.assert_contains(&EventMatcher::new().executable_path(TRUE_PATH))?;
    assert!(prefix.join("lib/bear/bin/bear-driver").is_file(), "driver not under LIBEXECDIR");
    assert!(!prefix.join("libexec").exists(), "default libexec directory was still created");
    Ok(())
}

/// A trailing slash on `LIBEXECDIR` must not reach the shipped entry
/// script as a `//` in the driver path.
#[test]
fn libexecdir_trailing_slash_is_dropped() -> Result<()> {
    let install = InstallEnvironment::with_libexecdir("lib/")?;
    let expected = format!("exec {}/bear/bin/bear-driver ", install.install_dir().join("lib").display());

    let sut = std::fs::read_to_string(install.path())?;

    assert!(sut.contains(&expected), "expected {expected:?} in the entry script:\n{sut}");
    Ok(())
}

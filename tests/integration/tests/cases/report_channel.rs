// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for the channel that carries the executions from the build to Bear.
//!
//! Each test runs a build script under Bear that compiles `test.c`, once per
//! interception mechanism.

#![cfg(unix)]

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

const MECHANISMS: [&str; 2] = ["wrapper", "preload"];

fn preload_available(mechanism: &str) -> bool {
    mechanism != "preload" || cfg!(has_preload_library)
}

/// A short directory in `/tmp`: the test directory can be too long for a socket address.
fn short_dir(prefix: &str) -> Result<tempfile::TempDir> {
    Ok(tempfile::Builder::new().prefix(prefix).tempdir_in("/tmp")?)
}

fn config_for(mechanism: &str, socket: &str) -> String {
    format!("schema: \"4.2\"\n\nintercept:\n  mode: {mechanism}\n{socket}")
}

/// Runs `build` under Bear with the given config and environment. Returns the output.
fn run(
    env: &TestEnvironment,
    config: &str,
    build: &str,
    variables: &[(&str, &std::path::Path)],
) -> Result<std::process::Output> {
    let script = env.create_shell_script("build.sh", build)?;
    let config_path = env.create_config(config)?;
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .env("CC", filename_of(COMPILER_C_PATH))
        .env("RUST_LOG", "info")
        .args(["--config", config_path.to_str().unwrap(), "--output", "compile_commands.json", "--"])
        .args([SHELL_PATH, script.to_str().unwrap()]);
    for (name, value) in variables {
        cmd.env(name, value);
    }
    Ok(cmd.output()?)
}

fn assert_one_entry(env: &TestEnvironment, output: &std::process::Output, case: &str) -> Result<()> {
    assert!(
        output.status.success(),
        "case {case}: bear failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let db = env.load_compilation_database("compile_commands.json")?;
    let count = db
        .entries()
        .iter()
        .filter(|entry| entry.get("file").and_then(|v| v.as_str()) == Some("test.c"))
        .count();
    assert_eq!(count, 1, "case {case}: {:?}", db.entries());
    Ok(())
}

/// The socket directories Bear left in `dir`. Other programs (ccache) may use it too.
fn bear_entries(dir: &std::path::Path) -> Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        count += usize::from(entry?.file_name().to_string_lossy().starts_with("bear-"));
    }
    Ok(count)
}

/// Tells whether `command` runs here. For the tools a sandbox may refuse at run time.
#[cfg(any(has_executable_setpriv, has_executable_unshare))]
fn command_works(command: &str, args: &[&str]) -> bool {
    std::process::Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(all(has_executable_setpriv, has_preload_library))]
fn entry_count(env: &TestEnvironment) -> Result<usize> {
    let db = env.load_compilation_database("compile_commands.json")?;
    Ok(db.entries().len())
}

/// Sets a mode and puts the old one back on drop: the install directory is shared.
#[cfg(all(has_executable_setpriv, has_preload_library))]
struct RestoreMode {
    path: std::path::PathBuf,
    old: std::fs::Permissions,
}

#[cfg(all(has_executable_setpriv, has_preload_library))]
impl RestoreMode {
    fn set(path: &std::path::Path, mode: u32) -> Result<Self> {
        use std::os::unix::fs::PermissionsExt;

        let old = std::fs::metadata(path)?.permissions();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
        Ok(Self { path: path.to_path_buf(), old })
    }
}

#[cfg(all(has_executable_setpriv, has_preload_library))]
impl Drop for RestoreMode {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, self.old.clone());
    }
}

/// Bear makes a private socket directory under `XDG_RUNTIME_DIR` for the time of the
/// build, and leaves nothing behind. The access is set, so root and other users see the same.
// Requirements: interception-report-channel
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell, has_executable_ls))]
fn socket_is_private_and_removed_after_the_build() -> Result<()> {
    for mechanism in MECHANISMS {
        if !preload_available(mechanism) {
            continue;
        }
        let env = TestEnvironment::new(&format!("report_socket_private_{mechanism}"))?;
        env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
        let runtime = short_dir("br")?;
        let config = config_for(mechanism, "  socket:\n    access: owner\n");
        let build = "ls -ld \"$XDG_RUNTIME_DIR\"/bear-* > listing.txt\n\
                     ls -l \"$XDG_RUNTIME_DIR\"/bear-*/ >> listing.txt\n\
                     $CC -c test.c\n";

        let sut = run(&env, &config, build, &[("XDG_RUNTIME_DIR", runtime.path())])?;

        assert_one_entry(&env, &sut, mechanism)?;
        let listing = env.read_file("listing.txt")?;
        let directories: Vec<_> = listing.lines().filter(|line| line.starts_with('d')).collect();
        assert_eq!(directories.len(), 1, "case {mechanism}: {listing}");
        assert!(directories[0].starts_with("drwx------"), "case {mechanism}: {listing}");
        assert!(
            listing.lines().any(|line| line.starts_with('s') && line.ends_with(" sock")),
            "case {mechanism}: {listing}"
        );
        assert_eq!(bear_entries(runtime.path())?, 0, "case {mechanism}: left behind");
    }
    Ok(())
}

/// A runtime directory that is too long for a socket address is skipped, and the build is
/// captured through the next base.
// Requirements: interception-report-channel
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn long_runtime_directory_is_skipped() -> Result<()> {
    for mechanism in MECHANISMS {
        if !preload_available(mechanism) {
            continue;
        }
        let env = TestEnvironment::new(&format!("report_socket_long_{mechanism}"))?;
        env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
        let runtime = short_dir(&"l".repeat(120))?;
        let config = config_for(mechanism, "");

        let sut = run(&env, &config, "$CC -c test.c\n", &[("XDG_RUNTIME_DIR", runtime.path())])?;

        assert_one_entry(&env, &sut, mechanism)?;
        assert_eq!(bear_entries(runtime.path())?, 0, "case {mechanism}: long directory used");
    }
    Ok(())
}

/// A build step in its own network namespace has no loopback, but shares the file system.
// Requirements: interception-report-channel
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell, has_executable_unshare))]
fn build_in_a_network_namespace_is_captured() -> Result<()> {
    if !command_works("unshare", &["-rn", "true"]) {
        eprintln!("skipped: `unshare -rn` is not allowed here");
        return Ok(());
    }
    for mechanism in MECHANISMS {
        if !preload_available(mechanism) {
            continue;
        }
        let env = TestEnvironment::new(&format!("report_socket_netns_{mechanism}"))?;
        env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
        let config = config_for(mechanism, "");

        let sut = run(&env, &config, "unshare -rn $CC -c test.c\n", &[])?;

        assert_one_entry(&env, &sut, mechanism)?;
    }
    Ok(())
}

/// A root Bear opens the socket to every user, so a build step that drops to another user
/// still reports. The `owner` setting closes it, and the build still works.
// Requirements: interception-report-channel
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell, has_executable_setpriv, has_preload_library))]
fn root_build_that_drops_to_another_user() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // SAFETY: geteuid has no arguments and cannot fail.
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("skipped: not running as root");
        return Ok(());
    }
    const DROP_USER: [&str; 3] = ["--reuid=65534", "--regid=65534", "--clear-groups"];
    if !command_works("setpriv", &[DROP_USER.as_slice(), &["true"]].concat()) {
        eprintln!("skipped: setpriv cannot drop privileges here");
        return Ok(());
    }
    let cases = [("default", "", 1), ("owner", "  socket:\n    access: owner\n", 0)];
    for (name, socket, expected) in cases {
        let env = TestEnvironment::new(&format!("report_socket_drop_user_{name}"))?;
        env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
        std::fs::set_permissions(env.test_dir(), std::fs::Permissions::from_mode(0o777))?;
        // The other user must reach the preload library in the install directory.
        let install_dir = env.bear_path().parent().and_then(std::path::Path::parent).unwrap();
        let _restore = RestoreMode::set(install_dir, 0o755)?;
        let config = config_for("preload", socket);
        // `XDG_RUNTIME_DIR` is 0700, so the other user cannot enter it: Bear must use `/tmp`.
        let runtime = short_dir("br")?;

        let sut = run(
            &env,
            &config,
            &format!("export CCACHE_DISABLE=1\nsetpriv {} $CC -c test.c\n", DROP_USER.join(" ")),
            &[("XDG_RUNTIME_DIR", runtime.path())],
        )?;

        assert!(sut.status.success(), "case {name}: {}", String::from_utf8_lossy(&sut.stderr));
        assert_eq!(entry_count(&env)?, expected, "case {name}");
    }
    Ok(())
}

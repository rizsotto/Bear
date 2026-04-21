// SPDX-License-Identifier: GPL-3.0-or-later

//! Intercept functionality tests for Bear integration
//!
//! These tests verify that Bear's command interception works correctly
//! across different scenarios, ported from the Python/lit test suite.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;
#[allow(unused_imports)]
use encoding_rs;

/// Test basic command interception with preload mechanism
// Requirements: interception-preload-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(has_executable_compiler_c)]
fn basic_command_interception() -> Result<()> {
    let env = TestEnvironment::new("basic_intercept")?;
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Run intercept mode to capture commands
    env.run_bear_success(&["intercept", "--output", "events.json", "--", COMPILER_C_PATH, "-c", "test.c"])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should have at least one event
    assert!(!events.events().is_empty());

    // Should contain compiler execution event
    let compiler_matcher = event_matcher!(executable_path: COMPILER_C_PATH.to_string());
    events.assert_contains(&compiler_matcher)?;

    Ok(())
}

/// Test shell command interception
// Requirements: interception-preload-mechanism
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_compiler_c, has_executable_echo, has_executable_shell))]
fn shell_command_interception() -> Result<()> {
    let env = TestEnvironment::new("shell_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create shell script that runs compiler
    let build_commands = [
        format!("{} \"Starting build...\"", ECHO_PATH),
        format!("{} -c test.c -o test.o", COMPILER_C_PATH),
        format!("{} \"Build complete\"", ECHO_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Intercept shell script execution
    env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should have captured events
    assert!(!events.events().is_empty());

    // Should capture shell execution
    let shell_matcher = event_matcher!(executable_name: "sh".to_string());
    events.assert_contains(&shell_matcher)?;

    // Should capture compiler execution
    let compiler_matcher = event_matcher!(executable_path: COMPILER_C_PATH.to_string());
    events.assert_contains(&compiler_matcher)?;

    Ok(())
}

/// Test shell commands without shebang
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_echo, has_executable_shell))]
fn shell_commands_without_shebang() -> Result<()> {
    let env = TestEnvironment::new("shell_no_shebang")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create shell script WITHOUT shebang
    let shell_script = [
        format!("{} \"Building without shebang...\"", ECHO_PATH),
        format!("{} -c test.c", COMPILER_C_PATH),
        format!("{} \"Done\"", ECHO_PATH),
    ]
    .join("\n");
    let script_path = env.create_build_script("build_no_shebang.sh", &shell_script)?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should still capture commands even without shebang
    events.assert_min_count(1)?;

    Ok(())
}

/// Test parallel command interception
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn parallel_command_interception() -> Result<()> {
    let env = TestEnvironment::new("parallel_intercept")?;

    // Create multiple source files
    for i in 1..=4 {
        env.create_source_files(&[(
            &format!("test_{}.c", i),
            &format!("int func_{}() {{ return {}; }}", i, i),
        )])?;
    }

    // Create parallel build script
    let build_commands = [
        format!("{} -c test_1.c &", filename_of(COMPILER_C_PATH)),
        format!("{} -c test_2.c &", filename_of(COMPILER_C_PATH)),
        format!("{} -c test_3.c &", filename_of(COMPILER_C_PATH)),
        format!("{} -c test_4.c &", filename_of(COMPILER_C_PATH)),
        "wait".to_string(),
    ]
    .join("\n");
    let script_path = env.create_shell_script("parallel_build.sh", &build_commands)?;

    env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should have captured all 4 parallel compiler invocations
    let compiler_matcher = event_matcher!(executable_name: filename_of(COMPILER_C_PATH));
    events.assert_count_matching(&compiler_matcher, 4)?;

    // Verify each individual compiler invocation was captured
    for i in 1..=4 {
        let specific_compiler_matcher = EventMatcher::new().arguments(vec![
            filename_of(COMPILER_C_PATH).to_string(),
            "-c".to_string(),
            format!("test_{}.c", i),
        ]);
        events.assert_contains(&specific_compiler_matcher)?;
    }

    Ok(())
}

/// Test build stdout capture during interception
#[test]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn build_stdout_capture() -> Result<()> {
    let env = TestEnvironment::new("stdout_capture")?;

    // Create script that outputs to stdout
    let script_commands = [
        format!("\"{}\" \"This goes to stdout\"", ECHO_PATH),
        format!("\"{}\" \"This also goes to stdout\"", ECHO_PATH),
        format!("\"{}\"", TRUE_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("stdout_test.sh", &script_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Stdout should be preserved
    assert!(output.stdout().contains("This goes to stdout"));
    assert!(output.stdout().contains("This also goes to stdout"));

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Events should still be captured
    events.assert_min_count(1)?;

    Ok(())
}

/// Test build stderr capture during interception
#[test]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn build_stderr_capture() -> Result<()> {
    let env = TestEnvironment::new("stderr_capture")?;

    // Create script that outputs to stderr
    let script_commands = [
        format!("\"{}\" \"This goes to stderr\" >&2", ECHO_PATH),
        format!("\"{}\" \"This also goes to stderr\" >&2", ECHO_PATH),
        format!("\"{}\"", TRUE_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("stderr_test.sh", &script_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Stderr should be preserved in the stderr stream
    assert!(output.stderr().contains("This goes to stderr"));
    assert!(output.stderr().contains("This also goes to stderr"));

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Events should still be captured
    events.assert_min_count(1)?;

    Ok(())
}

/// Test interception with empty environment
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_env))]
fn intercept_empty_environment() -> Result<()> {
    let env = TestEnvironment::new("empty_env_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Run with minimal environment
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        ENV_PATH,
        "-i",
        "PATH=/usr/bin:/bin",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should still capture execution even with empty environment
    events.assert_min_count(1)?;

    Ok(())
}

/// Test libtool command interception
///
/// Note: This test might be fragile test, because libtool versions are different.
/// eg.: MacOS CI was failing to complain about "unknown option character `-' in: --mode=compile".
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_libtool, has_executable_compiler_c))]
fn libtool_command_interception() -> Result<()> {
    let env = TestEnvironment::new("libtool_intercept")?;

    env.create_source_files(&[("lib.c", "int lib_func() { return 42; }")])?;

    // Use libtool to compile a library
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        LIBTOOL_PATH,
        "--mode=compile",
        "--tag=CC",
        COMPILER_C_PATH,
        "-c",
        "lib.c",
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should capture libtool and compiler invocations
    events.assert_min_count(1)?;

    // Should have captured libtool execution
    let libtool_matcher = EventMatcher::new().executable_name("libtool".to_string());
    events.assert_contains(&libtool_matcher)?;

    Ok(())
}

/// Build a PATH that excludes ccache directories and resolve the bare
/// compiler name within it. Without this, wrapper mode on a ccache-equipped
/// host recurses: `.bear/gcc` -> ccache -> PATH lookup for `gcc` -> `.bear/gcc`.
/// Same workaround used by `wrapper_mode_resolves_cc_bare_name_via_path`.
#[cfg(target_family = "unix")]
fn ccache_free_path_and_compiler() -> (std::ffi::OsString, std::path::PathBuf) {
    let safe_path = std::env::join_paths(
        std::env::split_paths(&std::env::var("PATH").unwrap_or_default())
            .filter(|p| !p.to_string_lossy().contains("ccache")),
    )
    .expect("failed to join PATH");
    let compiler_filename = filename_of(COMPILER_C_PATH);
    let real_compiler =
        which::which_in(&compiler_filename, Some(&safe_path), std::env::current_dir().unwrap())
            .unwrap_or_else(|_| std::path::PathBuf::from(COMPILER_C_PATH));
    (safe_path, real_compiler)
}

/// In wrapper mode, Bear creates a deterministic `.bear/` directory in the
/// working directory during the build and removes it automatically on exit.
/// Verify both: the build observes `.bear/` while it runs, and after Bear
/// returns the directory is gone.
// Requirements: interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn wrapper_mode_creates_and_cleans_up_bear_directory() -> Result<()> {
    let env = TestEnvironment::new("wrapper_bear_dir_cleanup")?;
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let (safe_path, real_compiler) = ccache_free_path_and_compiler();

    // Build script records whether `.bear/` was present during the build,
    // then invokes the compiler via $CC so the wrapper is exercised.
    let build = r#"if [ -d .bear ]; then echo present > bear_dir_status.txt; else echo missing > bear_dir_status.txt; fi
$CC -c test.c -o test.o
"#;
    let script = env.create_shell_script("build.sh", build)?;

    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: wrapper

compilers:
  - path: {}
"#,
        real_compiler.to_str().unwrap()
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir()).env("CC", filename_of(COMPILER_C_PATH)).env("PATH", &safe_path).args([
        "--config",
        config_path.to_str().unwrap(),
        "--output",
        "compile_commands.json",
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ]);
    let output = cmd.output()?;
    assert!(
        output.status.success(),
        "bear failed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let status = env.read_file("bear_dir_status.txt")?;
    assert_eq!(status.trim(), "present", "expected .bear/ to exist during the build, got {status:?}");

    let bear_dir = env.test_dir().join(".bear");
    assert!(
        !bear_dir.exists(),
        ".bear/ must be cleaned up after Bear exits but still present at {bear_dir:?}"
    );

    Ok(())
}

/// Two back-to-back wrapper-mode runs in the same working directory must each
/// create `.bear/` (deterministic name, not a random temp dir) and clean it
/// up. This matches the "deterministic directory" guarantee so that paths
/// recorded during `./configure` survive when a follow-on step re-runs Bear.
// Requirements: interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn wrapper_mode_bear_directory_is_deterministic_across_runs() -> Result<()> {
    let env = TestEnvironment::new("wrapper_bear_dir_deterministic")?;
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let (safe_path, real_compiler) = ccache_free_path_and_compiler();

    let build = r#"ls -d .bear >> bear_dir_observed.txt 2>/dev/null || echo missing >> bear_dir_observed.txt
$CC -c test.c -o test.o
"#;
    let script = env.create_shell_script("build.sh", build)?;

    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: wrapper

compilers:
  - path: {}
"#,
        real_compiler.to_str().unwrap()
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    for _ in 0..2 {
        let mut cmd = env.command_bear();
        cmd.current_dir(env.test_dir()).env("CC", filename_of(COMPILER_C_PATH)).env("PATH", &safe_path).args(
            [
                "--config",
                config_path.to_str().unwrap(),
                "--output",
                "compile_commands.json",
                "--",
                SHELL_PATH,
                script.to_str().unwrap(),
            ],
        );
        let output = cmd.output()?;
        assert!(
            output.status.success(),
            "bear failed on iteration: stdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!env.test_dir().join(".bear").exists(), ".bear/ must be cleaned up after each Bear run");
    }

    // Each run logged a line with the observed directory. Both lines must be
    // the deterministic `.bear/` form (no random temp dir), confirming the
    // name is stable across invocations.
    let observed = env.read_file("bear_dir_observed.txt")?;
    let lines: Vec<&str> = observed.lines().collect();
    assert_eq!(lines.len(), 2, "expected two observation lines, got {observed:?}");
    for line in &lines {
        assert_eq!(*line, ".bear", "wrapper directory must be the deterministic `.bear`, got {line:?}");
    }

    Ok(())
}

/// Test wrapper-based interception
// Requirements: interception-wrapper-mechanism
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_compiler_c, has_executable_echo))]
fn wrapper_based_interception() -> Result<()> {
    let env = TestEnvironment::new("wrapper_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    // Create a wrapper script
    let wrapper_commands = format!(
        r#"{} "Wrapper called with: $@"
exec {} "$@""#,
        ECHO_PATH, COMPILER_C_PATH
    );

    let wrapper_path = env.create_shell_script("cc-wrapper", &wrapper_commands)?;

    // Test with wrapper-based interception (when preload isn't available)
    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        wrapper_path.to_str().unwrap(),
        "-c",
        "test.c",
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should capture wrapper execution
    events.assert_min_count(1)?;

    // Should contain wrapper execution
    let wrapper_matcher = EventMatcher::new().executable_name("cc-wrapper".to_string());
    events.assert_contains(&wrapper_matcher)?;

    Ok(())
}

/// Test Unicode handling in shell commands
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn unicode_shell_commands() -> Result<()> {
    let env = TestEnvironment::new("unicode_intercept")?;

    // Create script with Unicode content
    let unicode_commands = [
        format!("\"{}\" \"Testing Unicode: αβγδε 中文 🚀\"", ECHO_PATH),
        format!("\"{}\" \"Файл с русскими именами\"", ECHO_PATH),
        format!("\"{}\"", TRUE_PATH),
    ]
    .join("\n");

    let script_path = env.create_shell_script("unicode_test.sh", &unicode_commands)?;

    let output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Should handle Unicode properly
    assert!(output.stdout().contains("αβγδε"));
    assert!(output.stdout().contains("中文"));
    assert!(output.stdout().contains("🚀"));

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Events should be captured properly despite Unicode content
    events.assert_min_count(2)?;

    Ok(())
}

/// Test interception with ISO-8859-2 encoding
///
/// This test verifies that Bear can properly intercept commands from shell scripts
/// that are encoded in ISO-8859-2 (Latin-2) character encoding, which is commonly
/// used in Central and Eastern European languages.
///
/// The test:
/// 1. Creates a shell script with Polish characters (ąęłńóśźż) that exist in ISO-8859-2
/// 2. Ensures the script file is actually written with ISO-8859-2 encoding (not UTF-8)
/// 3. Verifies that Bear can intercept commands from such encoded scripts
/// 4. Confirms that the encoding doesn't interfere with event capture
///
/// This addresses real-world scenarios where legacy build systems or scripts
/// may use non-UTF-8 encodings, particularly in European development environments.
#[test]
#[cfg(has_preload_library)]
#[cfg(all(has_executable_shell, has_executable_echo, has_executable_true))]
fn iso8859_2_encoding() -> Result<()> {
    let env = TestEnvironment::new("iso8859_2")?;

    // Create script with ISO-8859-2 characters (Polish characters that exist in ISO-8859-2)
    // These specific characters (ąęłńóśźż) are chosen because they:
    // - Are valid in ISO-8859-2 encoding
    // - Would be encoded differently in UTF-8
    // - Represent common characters in Polish and other Central European languages
    let script_commands =
        [format!("\"{}\" 'Testing ISO-8859-2: ąęłńóśźż'", ECHO_PATH), format!("\"{}\"", TRUE_PATH)]
            .join("\n");

    // Create the script with ISO-8859-2 encoding (not UTF-8)
    let script_path =
        env.create_shell_script_with_encoding("iso_test.sh", &script_commands, encoding_rs::ISO_8859_2)?;

    // Verify the script file is actually encoded in ISO-8859-2
    // This is the key improvement: we now actually verify the encoding is correct
    assert!(
        env.verify_file_encoding(&script_path, encoding_rs::ISO_8859_2)?,
        "Script file should be encoded in ISO-8859-2"
    );

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        SHELL_PATH,
        script_path.to_str().unwrap(),
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should handle encoding properly - Bear should intercept commands successfully
    // regardless of the script's character encoding
    events.assert_min_count(2)?;

    Ok(())
}

/// Test Valgrind integration
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_valgrind, has_executable_compiler_c))]
fn valgrind_integration() -> Result<()> {
    let env = TestEnvironment::new("valgrind_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        VALGRIND_PATH,
        "--tool=memcheck",
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should capture valgrind execution
    events.assert_min_count(1)?;

    // Should contain valgrind execution
    let valgrind_matcher = EventMatcher::new().executable_name("valgrind".to_string());
    events.assert_contains(&valgrind_matcher)?;

    Ok(())
}

/// Test fakeroot integration
#[test]
#[cfg(target_family = "unix")]
#[cfg(all(has_executable_fakeroot, has_executable_compiler_c))]
fn fakeroot_integration() -> Result<()> {
    let env = TestEnvironment::new("fakeroot_intercept")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let _output = env.run_bear_success(&[
        "intercept",
        "--output",
        "events.json",
        "--",
        FAKEROOT_PATH,
        COMPILER_C_PATH,
        "-c",
        "test.c",
    ])?;

    // Load and verify events
    let events = env.load_events_file("events.json")?;

    // Should capture fakeroot execution
    events.assert_min_count(1)?;

    // Should contain fakeroot execution
    let fakeroot_matcher = EventMatcher::new().executable_name("fakeroot".to_string());
    events.assert_contains(&fakeroot_matcher)?;

    Ok(())
}

/// Test that wrapper mode resolves bare compiler names from CC env var via PATH.
///
/// Covers the PATH resolution part of issue #686: when CC is set to a bare name
/// (e.g. "gcc" instead of "/usr/bin/gcc"), Bear's wrapper mode should resolve it
/// through PATH before registering wrapper targets.
///
/// The build script uses $CC so the wrapper actually intercepts it. To avoid
/// ccache recursion (where the wrapper calls ccache which finds the wrapper
/// again via PATH), we construct a PATH containing only the real compiler
/// directory, excluding any ccache directories.
// Requirements: interception-wrapper-mechanism
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn wrapper_mode_resolves_cc_bare_name_via_path() -> Result<()> {
    let env = TestEnvironment::new("wrapper_cc_bare_name")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let compiler_filename = filename_of(COMPILER_C_PATH);

    // Build a PATH that excludes ccache directories to avoid wrapper recursion
    // (ccache symlinks search PATH for the real compiler, finding the wrapper).
    let safe_path = std::env::join_paths(
        std::env::split_paths(&std::env::var("PATH").unwrap_or_default())
            .filter(|p| !p.to_string_lossy().contains("ccache")),
    )
    .expect("failed to join PATH");
    // Ensure the real compiler is reachable: resolve the compiler filename
    // in the safe PATH to get a non-ccache path (e.g. /usr/bin/gcc).
    let real_compiler = which::which_in(&compiler_filename, Some(&safe_path), env.test_dir())
        .unwrap_or_else(|_| std::path::PathBuf::from(COMPILER_C_PATH));
    let real_compiler_str = real_compiler.to_str().unwrap();

    // Build script uses $CC so the wrapper intercepts the call.
    let build_commands = "$CC -c test.c".to_string();
    let script_path = env.create_shell_script("build.sh", &build_commands)?;

    // Config forces wrapper mode and lists the real (non-ccache) compiler
    // to suppress PATH-based discovery.
    let config = format!(
        r#"
schema: "4.1"

intercept:
  mode: wrapper

compilers:
  - path: {}
"#,
        real_compiler_str
    );
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, &config)?;

    // Run the full bear pipeline with CC set to the bare compiler name (no path).
    // Bear must resolve "gcc" via PATH before creating wrapper symlinks.
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .env("CC", &compiler_filename)
        .env("PATH", &safe_path)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--output",
            "compile_commands.json",
            "--",
            SHELL_PATH,
            script_path.to_str().unwrap(),
        ]);

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify Bear resolved the bare CC name: the CC env var should be
    // overridden to point to a wrapper in the .bear directory. Without
    // PATH resolution, this would fail with "Executable not found".
    assert!(
        !stderr.contains("Skipping compiler env var CC="),
        "Bear should resolve CC={} via PATH, but it was skipped:\n{}",
        compiler_filename,
        stderr,
    );

    // Build should succeed and produce a compilation database entry.
    assert!(output.status.success(), "bear failed:\n{}", stderr);

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    Ok(())
}

/// Test that wrapper mode handles CC without .exe extension on Windows.
///
/// Reproduces the exact user scenario from issue #686: CC=cl (no extension,
/// no path) when cl.exe exists on PATH. On Windows, the OS may resolve the
/// wrapper executable name with different casing (e.g. "cl.EXE"), so the
/// wrapper config lookup must be case-insensitive and extension-agnostic.
#[test]
#[cfg(target_family = "windows")]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn wrapper_mode_resolves_cc_without_exe_extension_on_windows() -> Result<()> {
    let env = TestEnvironment::new("wrapper_cc_no_exe_ext")?;

    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let compiler_filename = filename_of(COMPILER_C_PATH);
    // Strip .exe extension to mimic CC=cl (the exact user scenario)
    let bare_name = compiler_filename
        .strip_suffix(".exe")
        .or_else(|| compiler_filename.strip_suffix(".EXE"))
        .unwrap_or(&compiler_filename);

    // Build script uses %CC% on Windows
    let build_commands = "%CC% -c test.c".to_string();
    let script_path = env.test_dir().join("build.bat");
    std::fs::write(&script_path, &build_commands)?;

    let config = r#"
schema: "4.1"

intercept:
  mode: wrapper
"#;
    let config_path = env.test_dir().join("config.yaml");
    std::fs::write(&config_path, config)?;

    // CC is set to bare name WITHOUT .exe (e.g. "cl" or "gcc")
    let mut cmd = env.command_bear();
    cmd.current_dir(env.test_dir())
        .env("RUST_LOG", "debug")
        .env("RUST_BACKTRACE", "1")
        .env("CC", bare_name)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--output",
            "compile_commands.json",
            "--",
            "cmd",
            "/c",
            script_path.to_str().unwrap(),
        ]);

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "bear failed:\n{}", stderr);

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    Ok(())
}

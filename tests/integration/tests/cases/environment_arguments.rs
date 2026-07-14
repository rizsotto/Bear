// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for folding compiler environment variables into entry arguments.
//!
//! Exercise `format.arguments.from_environment` end to end: a build that sets
//! `CPATH`, analysed with the option enabled (the default) and disabled.
//!
//! The whole module is gated (at its declaration in `cases/mod.rs`) on a
//! preload library plus a C compiler and shell, since every test needs all
//! three.

use crate::fixtures::constants::*;
use crate::fixtures::infrastructure::*;
use anyhow::Result;

const SRC: &str = "int main() { return 0; }";

/// Config that toggles environment-derived flags. `from_environment` is left at
/// its default (`true`) when `enabled` is `None`.
fn config(enabled: Option<bool>) -> String {
    let arguments = match enabled {
        Some(value) => format!("  arguments:\n    from_environment: {value}\n"),
        None => String::new(),
    };
    format!(
        r#"
schema: "4.1"
intercept:
  mode: preload
  path: "{}"
format:
  paths:
    directory: as-is
    file: as-is
{}"#,
        PRELOAD_LIBRARY_PATH, arguments
    )
}

/// Runs a `cc -c src.c -o src.o` build with `CPATH=/opt/envinc` exported, under
/// the given config, and returns the single entry's `arguments`.
fn arguments_with_cpath(name: &str, config_yaml: &str) -> Result<Vec<String>> {
    let env = TestEnvironment::new(name)?;
    env.create_source_files(&[("src.c", SRC)])?;
    let build = format!("export CPATH=/opt/envinc\n{} -c src.c -o src.o", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    env.run_bear_success(&[
        "--output",
        "compile_commands.json",
        "--config",
        env.create_config(config_yaml)?.to_str().unwrap(),
        "--",
        SHELL_PATH,
        script.to_str().unwrap(),
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let entry = db.entries()[0].clone();
    let arguments = entry
        .get("arguments")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    Ok(arguments)
}

// Requirements: output-env-derived-flags
#[test]
fn environment_flags_included_by_default() -> Result<()> {
    let sut = arguments_with_cpath("env_args_default", &config(None))?;

    assert!(
        sut.windows(2).any(|w| w == ["-I", "/opt/envinc"]),
        "CPATH should contribute -I /opt/envinc by default, got {sut:?}"
    );
    Ok(())
}

// Requirements: output-env-derived-flags
#[test]
fn environment_flags_excluded_when_disabled() -> Result<()> {
    let sut = arguments_with_cpath("env_args_disabled", &config(Some(false)))?;

    let expected = vec![
        COMPILER_C_PATH.to_string(),
        "-c".to_string(),
        "src.c".to_string(),
        "-o".to_string(),
        "src.o".to_string(),
    ];
    assert_eq!(sut, expected, "disabled from_environment must drop CPATH-derived flags");
    Ok(())
}

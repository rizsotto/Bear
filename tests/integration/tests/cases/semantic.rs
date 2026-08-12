use crate::fixtures::infrastructure::compilation_entry;
use crate::fixtures::*;
use anyhow::{Context, Result};
use serde_json::{Value, json};

#[test]
#[cfg(has_executable_compiler_c)]
fn basic_semantic_conversion() -> Result<()> {
    let env = TestEnvironment::new("basic_semantic")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create sample events file with compilation events using new format
    // Use proper JSON serialization to handle Windows paths with backslashes

    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event2 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = format!("{}\n{}", event1, event2);

    env.create_source_files(&[("events.json", &events_content), ("test.c", "int main() { return 0; }")])?;

    // Run semantic to convert events to compilation database
    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    // Verify compilation database was created
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the compilation entry matches expected format
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test.c".to_string()]
    ))?;

    Ok(())
}

#[test]
#[cfg(all(has_executable_compiler_c, has_executable_compiler_cxx))]
fn semantic_multiple_entries() -> Result<()> {
    let env = TestEnvironment::new("semantic_multiple")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events file with multiple compilation events using new format
    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test1.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event2 = json!({
        "executable": COMPILER_CXX_PATH,
        "arguments": [COMPILER_CXX_PATH, "-c", "test2.cpp"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event3 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test3.c", "-o", "test3.o"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = format!("{}\n{}\n{}", event1, event2, event3);

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test1.c", "int func1() { return 1; }"),
        ("test2.cpp", "int func2() { return 2; }"),
        ("test3.c", "int func3() { return 3; }"),
    ])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(3)?;

    // Verify all compilation entries
    db.assert_contains(&compilation_entry!(
        file: "test1.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test1.c".to_string()]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test2.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_CXX_PATH.to_string(), "-c".to_string(), "test2.cpp".to_string()]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test3.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test3.c".to_string(), "-o".to_string(), "test3.o".to_string()]
    ))?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_format_conversion() -> Result<()> {
    let env = TestEnvironment::new("semantic_format")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events with compiler flags
    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "-Wall", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = event1.to_string();

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "#include <stdio.h>\nint main() { printf(\"Hello\\n\"); return 0; }"),
    ])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the compilation entry preserves compiler flags
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "-Wall".to_string(),
            "test.c".to_string()
        ]
    ))?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_relative_paths() -> Result<()> {
    let env = TestEnvironment::new("semantic_relative_paths")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events with relative paths
    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "./src/main.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = event1.to_string();

    env.create_source_files(&[("events.json", &events_content), ("src/main.c", "int main() { return 0; }")])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify relative paths are handled correctly
    db.assert_contains(&compilation_entry!(
        file: "./src/main.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "./src/main.c".to_string()
        ]
    ))?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_wrapper_flags() -> Result<()> {
    let env = TestEnvironment::new("semantic_wrapper")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events with wrapper that adds flags
    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-DWRAPPER_FLAG", "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = event1.to_string();

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "#ifdef WRAPPER_FLAG\nint main() { return 0; }\n#endif"),
    ])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify wrapper flags are preserved
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-DWRAPPER_FLAG".to_string(),
            "-c".to_string(),
            "test.c".to_string()
        ]
    ))?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_clang_plugins() -> Result<()> {
    let env = TestEnvironment::new("semantic_clang_plugins")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events with clang plugin flags
    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-fplugin=libexample.so", "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = event1.to_string();

    env.create_source_files(&[("events.json", &events_content), ("test.c", "int main() { return 0; }")])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify plugin flags are preserved
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-fplugin=libexample.so".to_string(),
            "-c".to_string(),
            "test.c".to_string()
        ]
    ))?;

    Ok(())
}

#[test]
#[cfg(all(has_executable_compiler_c, has_executable_ls))]
fn semantic_with_filtering() -> Result<()> {
    let env = TestEnvironment::new("semantic_filtering")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events with both compilation and non-compilation commands
    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event2 = json!({
        "executable": LS_PATH,
        "arguments": [LS_PATH, "-la"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event3 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test2.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = format!("{}\n{}\n{}", event1, event2, event3);

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "int main() { return 0; }"),
        ("test2.c", "int func() { return 1; }"),
    ])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Should only contain the 2 compilation commands, not the ls command
    db.assert_count(2)?;

    // Verify only compilation entries are included
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test.c".to_string()]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test2.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test2.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: cli-exit-codes
#[test]
fn semantic_empty_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_empty")?;

    env.create_source_files(&[("events.json", "")])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: interception-events-format, cli-exit-codes
//
// Every one of these three lines is rejected (an unterminated object, a
// valid-but-incomplete object missing required fields, and another
// unterminated object): non-empty input from which nothing parsed must
// exit non-zero rather than silently succeed with an empty database.
#[test]
fn semantic_malformed_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_malformed")?;

    env.create_source_files(&[(
        "events.json",
        r#"{"invalid": "json"
{}
{malformed json"#,
    )])?;

    let result =
        env.run_bear(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;
    result.assert_failure()?;

    Ok(())
}

// Requirements: interception-events-format
//
// The same set of events in any order must yield a database with the same
// set of entries. Three distinct compilations consumed forward and reversed
// must produce identical entry sets; only their order in the file may
// differ. The bare `gcc` executable with an empty environment keeps the
// events host-independent: recognition is by name and the default path
// format never touches the filesystem.
#[test]
fn semantic_database_entry_set_is_order_independent() -> Result<()> {
    let env = TestEnvironment::new("semantic_order_independent")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let events: Vec<String> = ["a.c", "b.c", "c.c"]
        .iter()
        .map(|source| {
            json!({
                "executable": "gcc",
                "arguments": ["gcc", "-c", source],
                "working_dir": temp_dir,
                "environment": {}
            })
            .to_string()
        })
        .collect();
    let forward = events.join("\n") + "\n";
    let reverse = events.iter().rev().cloned().collect::<Vec<_>>().join("\n") + "\n";
    env.create_source_files(&[("forward.json", &forward), ("reverse.json", &reverse)])?;

    env.run_bear_success(&["semantic", "--input", "forward.json", "--output", "forward_db.json"])?;
    env.run_bear_success(&["semantic", "--input", "reverse.json", "--output", "reverse_db.json"])?;

    let mut sut_forward = env.load_compilation_database("forward_db.json")?.entries().to_vec();
    let mut sut_reverse = env.load_compilation_database("reverse_db.json")?.entries().to_vec();
    sut_forward.sort_by_key(|entry| entry["file"].as_str().map(String::from));
    sut_reverse.sort_by_key(|entry| entry["file"].as_str().map(String::from));
    assert_eq!(sut_forward.len(), 3, "each distinct compilation must yield an entry");
    assert_eq!(sut_forward, sut_reverse, "entry sets must match regardless of event order");

    Ok(())
}

// Requirements: output-json-compilation-database
//
// The compiler is written into `arguments[0]` exactly as the event observed
// it: a bare `gcc` stays `gcc`. Semantic analysis never resolves the
// executable against PATH (an earlier resolver leaked absolute -- even
// ccache-masquerade -- paths into the database and broke clangd's
// --query-driver allowlists). The event's PATH names a directory that
// really contains a resolvable `gcc`, so this test fails if resolution is
// ever reintroduced and leaks into the output.
#[test]
fn semantic_bare_compiler_name_stays_bare_in_output() -> Result<()> {
    let env = TestEnvironment::new("semantic_bare_name_stays_bare")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    std::fs::create_dir_all(env.test_dir().join("tools"))?;
    env.create_build_script("tools/gcc", "#!/bin/sh\nexit 0\n")?;
    let event = json!({
        "executable": "gcc",
        "arguments": ["gcc", "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": { "PATH": env.test_dir().join("tools").to_str().unwrap() }
    });
    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("test.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec!["gcc".to_string(), "-c".to_string(), "test.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: recognition-ambiguous-name-probe
//
// A masquerade link under an ambiguous name (a `cc` symlink into a ccache
// farm) is probed as invoked: ccache passes `--version` through to the
// underlying compiler, so the event classifies and yields an entry with
// the compiler recorded as observed. The test builds its own masquerade
// link from the host's ccache (located via the detected farm), so it does
// not depend on which names the host farm carries.
#[test]
#[cfg(all(unix, host_has_ccache_masquerade))]
fn masquerade_cc_event_yields_entry_with_observed_path() -> Result<()> {
    let env = TestEnvironment::new("masquerade_cc_event")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let farm = std::path::Path::new(env!("CCACHE_MASQUERADE_DIR"));
    let ccache = ["gcc", "cc", "g++", "c++", "clang", "clang++"]
        .iter()
        .find_map(|name| std::fs::canonicalize(farm.join(name)).ok())
        .context("no masquerade entry found in CCACHE_MASQUERADE_DIR")?;
    let masq_dir = env.test_dir().join("masq");
    std::fs::create_dir_all(&masq_dir)?;
    std::os::unix::fs::symlink(&ccache, masq_dir.join("cc"))?;
    let masq_cc = masq_dir.join("cc").to_str().unwrap().to_string();

    let event = json!({
        "executable": masq_cc,
        "arguments": [masq_cc, "-c", "hello.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("hello.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "hello.c".to_string(),
        directory: temp_dir.clone(),
        arguments: vec![masq_cc.clone(), "-c".to_string(), "hello.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: recognition-compiler-launchers
//
// A ccache masquerade link under a full compiler name (a `gcc` symlink
// into a ccache farm), invoked as `gcc -c main.c`, is treated as that
// compiler's compilation: the entry records the compiler as observed (the
// symlink path) with the invocation's own arguments. The underlying binary
// being ccache must not leak into the entry. Modeled on the ambiguous-name
// variant above (`masquerade_cc_event_yields_entry_with_observed_path`);
// here the name `gcc` is unambiguous, so recognition is by name alone.
#[test]
#[cfg(all(unix, host_has_ccache_masquerade))]
fn masquerade_gcc_event_yields_entry_with_observed_path() -> Result<()> {
    let env = TestEnvironment::new("masquerade_gcc_event")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let farm = std::path::Path::new(env!("CCACHE_MASQUERADE_DIR"));
    let ccache = ["gcc", "cc", "g++", "c++", "clang", "clang++"]
        .iter()
        .find_map(|name| std::fs::canonicalize(farm.join(name)).ok())
        .context("no masquerade entry found in CCACHE_MASQUERADE_DIR")?;
    let masq_dir = env.test_dir().join("masq");
    std::fs::create_dir_all(&masq_dir)?;
    std::os::unix::fs::symlink(&ccache, masq_dir.join("gcc"))?;
    let masq_gcc = masq_dir.join("gcc").to_str().unwrap().to_string();

    let event = json!({
        "executable": masq_gcc,
        "arguments": [masq_gcc, "-c", "main.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("main.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "main.c".to_string(),
        directory: temp_dir.clone(),
        arguments: vec![masq_gcc.clone(), "-c".to_string(), "main.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: output-compilation-entries
#[test]
#[cfg(all(has_executable_echo, has_executable_mkdir, has_executable_rm))]
fn semantic_non_compilation_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_non_compilation")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create events with only non-compilation commands
    let event1 = json!({
        "executable": ECHO_PATH,
        "arguments": ["hello"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event2 = json!({
        "executable": MKDIR_PATH,
        "arguments": ["-p", "build"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let event3 = json!({
        "executable": RM_PATH,
        "arguments": ["-f", "temp.txt"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = format!("{}\n{}\n{}", event1, event2, event3);

    env.create_source_files(&[("events.json", &events_content)])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Should contain no entries since none are compilation commands
    db.assert_count(0)?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_output_format() -> Result<()> {
    let env = TestEnvironment::new("semantic_output_format")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = event1.to_string();

    env.create_source_files(&[("events.json", &events_content), ("test.c", "int main() { return 0; }")])?;

    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the entry has the expected format with defines
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            COMPILER_C_PATH.to_string(),
            "-c".to_string(),
            "test.c".to_string()
        ]
    ))?;

    Ok(())
}

/// Regression test: all MSVC per-warning options documented on
/// <https://learn.microsoft.com/en-us/cpp/build/reference/compiler-option-warning-level>
/// accept their numeric value either glued (`/wd4995`) or separated by whitespace
/// (`/wd 4995`). Both forms are emitted by real `cl.exe` invocations and by
/// Makefiles in the wild (e.g. `CFLAGS = /wd 4995 /wd 4996 ...`). The separated
/// form must survive semantic analysis intact; dropping the number silently would
/// corrupt compile_commands.json and break downstream tools such as clangd
/// (emits `drv_invalid_int_value` per translation unit).
///
/// Covers `/w1`, `/w2`, `/w3`, `/w4` (set warning level for a specific warning)
/// and `/wd`, `/we`, `/wo` (disable / as-error / report-once).
///
/// This test is platform-independent: it exercises the `semantic` subcommand on
/// a hand-crafted events file and does not require a real `cl.exe` to be present.
#[test]
fn msvc_per_warning_options_preserve_separated_value() -> Result<()> {
    let env = TestEnvironment::new("msvc_per_warning_options_separated")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    // Use a bare "cl.exe" -- the recognizer matches on the filename stem only, so we
    // do not need the file to exist on disk. Keeps the test hermetic across platforms.
    let cl = "cl.exe";

    let event = json!({
        "executable": cl,
        "arguments": [
            cl,
            "/w1", "4100",
            "/w2", "4101",
            "/w3", "4102",
            "/w4", "4103",
            "/wd", "4995",
            "/we", "4996",
            "/wo", "4819",
            "/c", "test.c",
        ],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("test.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Each flag/value pair must round-trip with its numeric value intact. Before
    // the fix, these flags matched a prefix-only pattern, so the standalone
    // numeric token following each flag was reclassified as a source file and
    // dropped from the output.
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            cl.to_string(),
            "/w1".to_string(), "4100".to_string(),
            "/w2".to_string(), "4101".to_string(),
            "/w3".to_string(), "4102".to_string(),
            "/w4".to_string(), "4103".to_string(),
            "/wd".to_string(), "4995".to_string(),
            "/we".to_string(), "4996".to_string(),
            "/wo".to_string(), "4819".to_string(),
            "/c".to_string(),
            "test.c".to_string(),
        ]
    ))?;

    Ok(())
}

/// Regression test: `/Wv[:version]` has an optional value (cl uses the current
/// compiler version when omitted). Both forms -- bare `/Wv` and `/Wv:17` -- must
/// round-trip through semantic analysis without losing tokens or dropping the
/// entry.
#[test]
fn msvc_wv_optional_version_is_preserved() -> Result<()> {
    let env = TestEnvironment::new("msvc_wv_optional_version")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let cl = "cl.exe";

    // Two translation units, one per /Wv form, so the test exercises both paths
    // in a single run.
    let event_bare = json!({
        "executable": cl,
        "arguments": [cl, "/Wv", "/c", "bare.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let event_with_version = json!({
        "executable": cl,
        "arguments": [cl, "/Wv:17", "/c", "versioned.c"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events = format!("{}\n{}", event_bare, event_with_version);

    env.create_source_files(&[
        ("events.json", &events),
        ("bare.c", "int main(void) { return 0; }"),
        ("versioned.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;

    db.assert_contains(&compilation_entry!(
        file: "bare.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![cl.to_string(), "/Wv".to_string(), "/c".to_string(), "bare.c".to_string()]
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "versioned.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![cl.to_string(), "/Wv:17".to_string(), "/c".to_string(), "versioned.c".to_string()]
    ))?;

    Ok(())
}

/// Companion to `msvc_per_warning_options_preserve_separated_value`: the fix
/// switched /wd, /we, /wo from `FlagPattern::Prefix` to
/// `FlagPattern::ExactlyWithGluedOrSep`, so the glued path now runs through
/// different generated code than before. Lock the glued form behavior so a
/// future refactor of the pattern types cannot silently regress the common
/// cl.exe spelling (/wd4995, /w34326).
#[test]
fn msvc_per_warning_options_preserve_glued_value() -> Result<()> {
    let env = TestEnvironment::new("msvc_per_warning_options_glued")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let cl = "cl.exe";

    let event = json!({
        "executable": cl,
        "arguments": [
            cl,
            "/w14100", "/w24101", "/w34102", "/w44103",
            "/wd4995", "/we4996", "/wo4819",
            "/c", "test.c",
        ],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("test.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            cl.to_string(),
            "/w14100".to_string(), "/w24101".to_string(), "/w34102".to_string(), "/w44103".to_string(),
            "/wd4995".to_string(), "/we4996".to_string(), "/wo4819".to_string(),
            "/c".to_string(),
            "test.c".to_string(),
        ]
    ))?;

    Ok(())
}

/// `clang_cl.yaml` inherits the per-warning rules via `extends: msvc`. The
/// bear-codegen snapshot proves the generated flag array is correct, but does
/// not exercise the runtime matcher. This test drives the `semantic` subcommand
/// with a clang-cl executable and a mix of glued / separated / colon forms to
/// confirm the inheritance is effective end-to-end.
#[test]
fn clang_cl_inherits_msvc_per_warning_options() -> Result<()> {
    let env = TestEnvironment::new("clang_cl_inherits_msvc_per_warning")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let cl = "clang-cl.exe";

    let event = json!({
        "executable": cl,
        "arguments": [
            cl,
            "/wd4995",
            "/we", "4996",
            "/w3", "4102",
            "/w44103",
            "/Wv:17",
            "/c", "test.c",
        ],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("test.c", "int main(void) { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            cl.to_string(),
            "/wd4995".to_string(),
            "/we".to_string(), "4996".to_string(),
            "/w3".to_string(), "4102".to_string(),
            "/w44103".to_string(),
            "/Wv:17".to_string(),
            "/c".to_string(),
            "test.c".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: output-compilation-entries
//
// Regression test for issue #715: Clang's `-F` (framework search path) is a
// JoinedOrSeparate option, so `-F /Frameworks` and `-F/Frameworks` are the same
// invocation. Bear only recognized the joined spelling, so the separated operand
// fell through to source classification and was dropped, leaving a standalone
// `-F` that no compiler can consume. SwiftPM emits the separated form on macOS,
// which is how this surfaced.
//
// Both spellings are checked in one run, each on its own translation unit, so the
// pair cannot drift apart. `bear semantic` runs the interpreter without executing
// clang, so no toolchain is required and the executable name can be a bare `clang`.
#[test]
fn clang_framework_search_path_preserves_separated_and_joined_operand() -> Result<()> {
    let env = TestEnvironment::new("clang_framework_search_path")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let clang = "clang";

    let event_separated = json!({
        "executable": clang,
        "arguments": [clang, "-F", "/Frameworks", "-c", "separated.cpp"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let event_joined = json!({
        "executable": clang,
        "arguments": [clang, "-F/Frameworks", "-c", "joined.cpp"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events = format!("{}\n{}", event_separated, event_joined);

    env.create_source_files(&[
        ("events.json", &events),
        ("separated.cpp", "int main() { return 0; }"),
        ("joined.cpp", "int main() { return 0; }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;

    db.assert_contains(&compilation_entry!(
        file: "separated.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            clang.to_string(),
            "-F".to_string(), "/Frameworks".to_string(),
            "-c".to_string(),
            "separated.cpp".to_string(),
        ]
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "joined.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            clang.to_string(),
            "-F/Frameworks".to_string(),
            "-c".to_string(),
            "joined.cpp".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: output-compilation-entries
//
// Vala's `valac` is a transpiler-driver: it parses GNU-style (GOption) flags,
// many of which take a separate-token value (`--pkg gio-2.0`, `--basedir ../src`).
// Because Bear classifies any bare argument as a source file, an unrecognized
// value-consuming flag turns its value into a phantom source -- so the single
// strongest regression guard is that exactly ONE entry (for the `.vala` source)
// is produced. `bear semantic` runs the interpreter without executing valac, so
// no Vala toolchain is required and the executable name can be a bare `valac`.
#[test]
fn vala_transpile_mode_produces_single_entry() -> Result<()> {
    let env = TestEnvironment::new("vala_transpile_mode")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "valac",
        "arguments": [
            "valac", "--pkg", "gio-2.0", "--define=FOO",
            "-X", "-lm", "--basedir", "../src", "-C", "foo.vala"
        ],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[("events.json", &event.to_string()), ("foo.vala", "void main() { }")])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Exactly one entry: the separate-token values gio-2.0 and ../src did NOT
    // leak in as their own source entries.
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "foo.vala".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "valac".to_string(),
            "--pkg".to_string(), "gio-2.0".to_string(),
            "--define=FOO".to_string(),
            "-X".to_string(), "-lm".to_string(),
            "--basedir".to_string(), "../src".to_string(),
            "-C".to_string(),
            "foo.vala".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: output-compilation-entries
//
// The default (non-`-C`) driver mode still yields one entry for the `.vala`
// source. The internal C-compiler invocation that valac spawns at build time
// is a separate process and is not part of a `semantic`-mode events file, so it
// is out of scope here (see docs/rationale/vala-transpiler-database).
#[test]
fn vala_default_mode_produces_single_entry() -> Result<()> {
    let env = TestEnvironment::new("vala_default_mode")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "valac",
        "arguments": ["valac", "--pkg", "gio-2.0", "foo.vala"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[("events.json", &event.to_string()), ("foo.vala", "void main() { }")])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "foo.vala".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "valac".to_string(),
            "--pkg".to_string(), "gio-2.0".to_string(),
            "foo.vala".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: output-compilation-entries
//
// valac compiles all of a target's `.vala` sources together as one translation
// unit and produces one combined output. Bear must therefore emit exactly ONE
// entry per `valac` invocation, not one per source, with `file` set to the first
// source and every source retained in the command. This is the regression that
// proves the single-translation-unit behaviour; the single-source tests above
// are only N=1 guards. `bear semantic` runs the interpreter without executing
// valac, so no Vala toolchain is required.
#[test]
fn vala_multiple_sources_produce_single_entry() -> Result<()> {
    let env = TestEnvironment::new("vala_multiple_sources")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "valac",
        "arguments": [
            "valac", "--pkg", "gio-2.0", "--library", "foo",
            "a.vala", "b.vala", "c.vala"
        ],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("a.vala", "void a() { }"),
        ("b.vala", "void b() { }"),
        ("c.vala", "void main() { }"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Three sources, but valac is single-translation-unit: exactly one entry,
    // keyed on the first source, with all three sources retained.
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "a.vala".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "valac".to_string(),
            "--pkg".to_string(), "gio-2.0".to_string(),
            "--library".to_string(), "foo".to_string(),
            "a.vala".to_string(),
            "b.vala".to_string(),
            "c.vala".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: recognition-compiler-names
//
// An `mpicc -c hello.c` execution yields one entry, with the wrapper itself
// (not the underlying compiler) as the recorded compiler.
#[test]
fn mpi_wrapper_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("mpi_wrapper_execution", &["mpicc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names
//
// Wrapper-info invocations (`-showme`, `-show`, `-compile_info`) print the
// underlying compiler command and exit; none of them compile anything, so
// none of them should yield a database entry.
#[test]
fn mpi_wrapper_info_flags_yield_no_entry() -> Result<()> {
    let env = TestEnvironment::new("mpi_wrapper_info_flags")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    for flag in ["-showme", "-show", "-compile_info"] {
        let event = json!({
            "executable": "mpicc",
            "arguments": ["mpicc", flag],
            "working_dir": temp_dir,
            "environment": {}
        });

        env.create_source_files(&[("events.json", &event.to_string()), ("mpicc", "")])?;

        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

        let db = env.load_compilation_database("compile_commands.json")?;
        db.assert_count(0).with_context(|| format!("flag {flag} must yield no entry"))?;
    }

    Ok(())
}

// Requirements: recognition-compiler-names
//
// MPICH's compiler-override flag `-cc=gcc` must survive as a single token
// (not be split, not be expanded) and must not swallow the source file that
// follows it.
#[test]
fn mpi_wrapper_compiler_override_flag_is_retained() -> Result<()> {
    assert_driver_yields_single_entry("mpi_wrapper_compiler_override", &["mpicc", "-cc=gcc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names, output-duplicate-detection
//
// In preload mode the wrapper's child compiler exec is intercepted too, so a
// single compilation can produce both an `mpicc` and a `gcc` event for the
// same file. The default duplicate filter (directory+file) must collapse
// them to one entry, and since the wrapper's event comes first in the event
// stream, the surviving entry must record the wrapper invocation.
#[test]
fn mpi_wrapper_and_child_compiler_events_collapse_to_wrapper_entry() -> Result<()> {
    assert_duplicate_events_collapse_to_first(
        "mpi_wrapper_duplicate_collapse",
        &["mpicc", "-c", "hello.c"],
        &["gcc", "-c", "hello.c"],
    )
}

// Requirements: recognition-compiler-names
//
// A `craycc -c hello.c` execution yields one entry, using the CCE C/C++
// compiler name directly (the same shape as the existing Cray Fortran
// support for `crayftn`/`ftn`).
#[test]
fn cray_cc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("cray_cc_execution", &["craycc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names
//
// A `hipcc -c hello.c` execution yields one entry, using the ROCm HIP
// compiler driver name directly (parsed with Clang flag semantics).
#[test]
fn hipcc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("hipcc_execution", &["hipcc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names
//
// A `qcc -c hello.c` execution yields one entry, using the QNX driver name
// directly (parsed with GCC flag semantics -- QNX 8 is GCC-backed).
#[test]
fn qnx_qcc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("qnx_qcc_execution", &["qcc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names
//
// QNX's attached-value variant selector (`-Vgcc_ntoaarch64le`) must be
// treated as a driver option, never as an input file, and must be retained
// verbatim in the recorded arguments.
#[test]
fn qnx_qcc_variant_selector_is_retained_as_driver_option() -> Result<()> {
    assert_driver_yields_single_entry(
        "qnx_qcc_variant_selector",
        &["qcc", "-Vgcc_ntoaarch64le", "-c", "hello.c"],
    )
}

// Requirements: recognition-compiler-names
//
// A `tiarmclang -c hello.c` execution yields one entry (TI's clang-based
// driver, parsed with Clang flag semantics).
#[test]
fn ti_tiarmclang_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("ti_tiarmclang_execution", &["tiarmclang", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names
//
// An `xc8-cc -c hello.c` execution yields one entry (Microchip's gcc-styled
// XC8 driver, parsed with GCC flag semantics).
#[test]
fn microchip_xc8_cc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("microchip_xc8_cc_execution", &["xc8-cc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names
//
// An `emcc -c hello.c` execution yields one entry, using the Emscripten
// driver name directly (parsed with Clang flag semantics).
#[test]
fn emscripten_emcc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("emscripten_emcc_execution", &["emcc", "-c", "hello.c"])
}

// Requirements: recognition-compiler-names, output-duplicate-detection
//
// In preload mode emcc's underlying clang child process is intercepted too,
// so a single compilation can produce both an `emcc` and a `clang` event for
// the same file. The default duplicate filter (directory+file) must collapse
// them to one entry, and since the driver's event comes first in the event
// stream, the surviving entry must record the emcc invocation.
#[test]
fn emscripten_driver_and_clang_child_events_collapse_to_driver_entry() -> Result<()> {
    assert_duplicate_events_collapse_to_first(
        "emscripten_duplicate_collapse",
        &["emcc", "-c", "hello.c"],
        &["clang", "-c", "hello.c"],
    )
}

// Requirements: recognition-compiler-launchers
//
// An `icecc gcc -c hello.c` execution is recorded as the real compiler's
// compilation: the icecc token is dropped and gcc's argv survives, the same
// contract as ccache/distcc/sccache.
#[test]
fn icecc_launcher_execution_records_real_compiler() -> Result<()> {
    assert_launcher_execution_yields_entry(
        "icecc_launcher_execution",
        &["icecc", "gcc"],
        &["icecc", "gcc", "-c", "hello.c"],
        &["gcc", "-c", "hello.c"],
    )
}

// Requirements: recognition-compiler-names
//
// A `gfortran -c hello.f90` execution yields one entry for the Fortran
// source, with the invocation's arguments recorded verbatim -- one
// representative row proving the Fortran compiler names follow the same
// recognition pattern as the C family, at entry level.
#[test]
fn gfortran_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry_for_source(
        "gfortran_execution",
        &["gfortran", "-c", "hello.f90"],
        "hello.f90",
        "program hello\nend program hello\n",
    )
}

// Requirements: recognition-compiler-names
//
// A `nasm -f elf64 -o hello.o hello.asm` execution yields one entry for
// `hello.asm`, using the NASM driver name directly, with the invocation's
// arguments recorded verbatim.
#[test]
fn nasm_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry_for_source(
        "nasm_execution",
        &["nasm", "-f", "elf64", "-o", "hello.o", "hello.asm"],
        "hello.asm",
        "section .text\nglobal _start\n_start:\n    ret\n",
    )
}

// Requirements: recognition-compiler-names
//
// A `fasm hello.asm` execution yields one entry for `hello.asm`, using the
// flat assembler driver name directly.
#[test]
fn fasm_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry_for_source(
        "fasm_execution",
        &["fasm", "hello.asm"],
        "hello.asm",
        "org 0x100\nret\n",
    )
}

// Requirements: output-compilation-entries
//
// NASM's `-M` and `-MG` emit Makefile dependencies and stop: nothing is
// assembled, so a `nasm -M hello.asm` execution must yield no entry. Guards
// the stops_at_preprocessing classification in nasm.yaml.
#[test]
fn nasm_dependency_only_invocation_yields_no_entry() -> Result<()> {
    let env = TestEnvironment::new("nasm_dependency_only")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    for flag in ["-M", "-MG"] {
        let event = json!({
            "executable": "nasm",
            "arguments": ["nasm", flag, "hello.asm"],
            "working_dir": temp_dir,
            "environment": {}
        });

        env.create_source_files(&[
            ("events.json", &event.to_string()),
            ("nasm", ""),
            ("hello.asm", "section .text\nglobal _start\n_start:\n    ret\n"),
        ])?;

        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

        let db = env.load_compilation_database("compile_commands.json")?;
        db.assert_count(0).with_context(|| format!("flag {flag} must yield no entry"))?;
    }

    Ok(())
}

// Requirements: output-compilation-entries
//
// NASM's `-MD` generates the dependency file as a side effect of a real
// assembly step, so the invocation still yields its entry for `hello.asm`.
#[test]
fn nasm_dependency_side_effect_assembly_yields_entry() -> Result<()> {
    assert_driver_yields_single_entry_for_source(
        "nasm_dependency_side_effect",
        &["nasm", "-MD", "hello.d", "-f", "elf64", "-o", "hello.o", "hello.asm"],
        "hello.asm",
        "section .text\nglobal _start\n_start:\n    ret\n",
    )
}

// Requirements: recognition-compiler-names
//
// Direct assembly through a driver (`gcc -c foo.s`) is already recorded
// today via the driver's own entry -- this is the actual fix for the
// compile-then-assemble class of bug reported in issue #146, and it needs
// no new recognition code (the standalone-assembler recognizer added by
// this requirement is not involved). Locking this in guards against a
// future regression that would make it invisible again.
//
// Unlike the synthetic embedded-toolchain names used elsewhere in this
// file, `gcc` is a real, `PATH`-resolvable executable on any host that can
// build Bear (its own host requirements list a `cc` toolchain), which
// historically made `ExecutableResolver` rewrite the recorded compiler
// token to gcc's absolute path. The recorded compiler now keeps the
// observed spelling (see `semantic_bare_compiler_name_stays_bare_in_output`);
// this test still asserts only the parts it cares about: one entry, for
// `foo.s`, invoked with `-c foo.s`, as `gcc`.
#[test]
fn driver_compiled_assembly_yields_driver_entry() -> Result<()> {
    let env = TestEnvironment::new("driver_compiled_assembly")?;
    let temp_dir = env.test_dir().to_str().unwrap().to_string();

    let event = json!({
        "executable": "gcc",
        "arguments": ["gcc", "-c", "foo.s"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("foo.s", ".text\n.globl main\nmain:\n    ret\n"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    let entry = &db.entries()[0];
    assert_eq!(entry["file"], "foo.s");
    assert_eq!(entry["directory"], temp_dir);
    let arguments: Vec<String> = entry["arguments"]
        .as_array()
        .context("arguments must be an array")?
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(&arguments[1..], &["-c".to_string(), "foo.s".to_string()]);
    let compiler_basename = std::path::Path::new(&arguments[0]).file_name().and_then(|n| n.to_str());
    assert_eq!(compiler_basename, Some("gcc"), "recorded compiler must be gcc, got {:?}", arguments[0]);

    Ok(())
}

// Requirements: recognition-compiler-names
//
// A `swiftc -c hello.swift` execution yields one entry, using the Swift
// driver name directly, with the invocation's arguments recorded verbatim.
#[test]
fn swiftc_single_file_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry_for_source(
        "swiftc_single_file",
        &["swiftc", "-c", "hello.swift"],
        "hello.swift",
        "print(\"hello\")\n",
    )
}

// Requirements: recognition-compiler-names
//
// Swift's whole-module compilation names several sources in one invocation,
// but SourceKit-LSP looks up a compile command per file. Bear must therefore
// emit one entry PER source (unlike valac's single combined entry), and every
// one of those entries must carry the COMPLETE invocation -- all sources, not
// just its own -- because each file's semantics depend on the whole module.
// `bear semantic` runs the interpreter without executing swiftc, so no Swift
// toolchain is required.
#[test]
fn swiftc_whole_module_invocation_yields_one_entry_per_source_with_full_arguments() -> Result<()> {
    let env = TestEnvironment::new("swiftc_whole_module")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "swiftc",
        "arguments": ["swiftc", "-module-name", "App", "-emit-object", "a.swift", "b.swift"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("a.swift", "func a() {}\n"),
        ("b.swift", "func main() {}\n"),
        ("swiftc", ""),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Two sources, whole-module: exactly two entries (one per source), each
    // retaining every source and flag from the original invocation.
    db.assert_count(2)?;
    let full_arguments = vec![
        "swiftc".to_string(),
        "-module-name".to_string(),
        "App".to_string(),
        "-emit-object".to_string(),
        "a.swift".to_string(),
        "b.swift".to_string(),
    ];
    db.assert_contains(&compilation_entry!(
        file: "a.swift".to_string(),
        directory: temp_dir.to_string(),
        arguments: full_arguments.clone()
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "b.swift".to_string(),
        directory: temp_dir.to_string(),
        arguments: full_arguments
    ))?;

    Ok(())
}

// Requirements: recognition-compiler-names
//
// swiftc spawns per-file `swift-frontend` jobs the way gcc spawns `cc1`;
// the internal frontend executable must yield no database entry.
#[test]
fn swift_frontend_executable_execution_yields_no_entry() -> Result<()> {
    let env = TestEnvironment::new("swift_frontend_executable")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "swift-frontend",
        "arguments": ["swift-frontend", "-frontend", "-c", "hello.swift"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("hello.swift", "print(\"hello\")\n"),
        ("swift-frontend", ""),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: recognition-compiler-names
//
// A legacy toolchain that re-invokes itself as `swiftc -frontend` must also
// yield no entry, via the `ignore_when.flags` filter (mirrors clang's -cc1).
#[test]
fn swiftc_frontend_flag_execution_yields_no_entry() -> Result<()> {
    let env = TestEnvironment::new("swiftc_frontend_flag")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "swiftc",
        "arguments": ["swiftc", "-frontend", "-c", "hello.swift"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("hello.swift", "print(\"hello\")\n"),
        ("swiftc", ""),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: recognition-compiler-names
//
// `swiftc --version` prints version information and exits; it yields no
// database entry.
#[test]
fn swiftc_version_flag_yields_no_entry() -> Result<()> {
    let env = TestEnvironment::new("swiftc_version_flag")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "swiftc",
        "arguments": ["swiftc", "--version"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[("events.json", &event.to_string()), ("swiftc", "")])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: recognition-cpp20-modules
//
// A C++20 module-interface compile (`clang++ --precompile ... -o foo.pcm`)
// must produce exactly one entry for the interface source (`foo.cppm`), with
// `--precompile` and the input/output paths preserved verbatim. The `.pcm`
// output is consumed by the `-o` flag and must never itself surface as a
// `file` or a standalone source argument.
#[test]
fn clang_precompile_module_interface_yields_single_entry_for_source() -> Result<()> {
    let env = TestEnvironment::new("clang_precompile_module_interface")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "clang++",
        "arguments": ["clang++", "--precompile", "-std=c++20", "foo.cppm", "-o", "foo.pcm"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[("events.json", &event.to_string()), ("foo.cppm", "export module foo;\n")])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "foo.cppm".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "clang++".to_string(),
            "--precompile".to_string(),
            "-std=c++20".to_string(),
            "foo.cppm".to_string(),
            "-o".to_string(),
            "foo.pcm".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: recognition-cpp20-modules
//
// Consuming a precompiled module via `-fmodule-file=<name>=<file>` must not
// break recognition of the real source (`main.cpp`), and the referenced
// `.pcm` must never surface as its own entry.
#[test]
fn clang_module_file_flag_yields_single_entry_for_main_source() -> Result<()> {
    let env = TestEnvironment::new("clang_module_file_flag")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "clang++",
        "arguments": ["clang++", "-std=c++20", "-fmodule-file=foo=foo.pcm", "-c", "main.cpp"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[
        ("events.json", &event.to_string()),
        ("main.cpp", "import foo;\nint main() { return 0; }\n"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "main.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "clang++".to_string(),
            "-std=c++20".to_string(),
            "-fmodule-file=foo=foo.pcm".to_string(),
            "-c".to_string(),
            "main.cpp".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: recognition-cpp20-modules
//
// A mixed build with one module interface and one consumer, processed in a
// single semantic run, must yield exactly two entries: no cross-talk between
// the `--precompile` invocation and the `-fmodule-file=` invocation.
#[test]
fn clang_module_interface_and_consumer_yield_two_entries() -> Result<()> {
    let env = TestEnvironment::new("clang_module_interface_and_consumer")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event1 = json!({
        "executable": "clang++",
        "arguments": ["clang++", "--precompile", "-std=c++20", "foo.cppm", "-o", "foo.pcm"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let event2 = json!({
        "executable": "clang++",
        "arguments": ["clang++", "-std=c++20", "-fmodule-file=foo=foo.pcm", "-c", "main.cpp"],
        "working_dir": temp_dir,
        "environment": {}
    });

    let events_content = format!("{}\n{}", event1, event2);

    env.create_source_files(&[
        ("events.json", &events_content),
        ("foo.cppm", "export module foo;\n"),
        ("main.cpp", "import foo;\nint main() { return 0; }\n"),
    ])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;
    db.assert_contains(&compilation_entry!(
        file: "foo.cppm".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "clang++".to_string(),
            "--precompile".to_string(),
            "-std=c++20".to_string(),
            "foo.cppm".to_string(),
            "-o".to_string(),
            "foo.pcm".to_string(),
        ]
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "main.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "clang++".to_string(),
            "-std=c++20".to_string(),
            "-fmodule-file=foo=foo.pcm".to_string(),
            "-c".to_string(),
            "main.cpp".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: interception-events-format
//
// `bear semantic --input -` must read the event stream from standard input
// and produce the same compilation database as the equivalent file-backed
// run, so the pipeline `<producer> | bear semantic --input -` works.
#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_input_stdin_matches_file_input() -> Result<()> {
    let env = TestEnvironment::new("semantic_input_stdin")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let event2 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "other.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let events_content = format!("{}\n{}\n", event1, event2);

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "int main() { return 0; }"),
        ("other.c", "int main() { return 0; }"),
    ])?;

    // Baseline: the same event stream read from a file.
    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "from_file.json"])?;
    let from_file = env.load_compilation_database("from_file.json")?;
    from_file.assert_count(2)?;

    // Under test: the identical stream read from standard input via `-`.
    let stdin_result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "from_stdin.json"],
        events_content.as_bytes(),
    )?;
    stdin_result.assert_success()?;
    let from_stdin = env.load_compilation_database("from_stdin.json")?;

    let mut file_entries: Vec<String> = from_file.entries().iter().map(|entry| entry.to_string()).collect();
    let mut stdin_entries: Vec<String> = from_stdin.entries().iter().map(|entry| entry.to_string()).collect();
    file_entries.sort();
    stdin_entries.sort();

    assert_eq!(
        stdin_entries, file_entries,
        "semantic --input - must produce the same compilation database as semantic --input <file>"
    );

    Ok(())
}

// Requirements: interception-events-format
//
// `bear semantic` is a filter: with no input named it reads the event
// stream from standard input, so a producer pipes into it with no flags
// (`<producer> | bear semantic`).
#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_input_defaults_to_stdin() -> Result<()> {
    let env = TestEnvironment::new("semantic_default_stdin")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let result = env.run_bear_with_stdin(
        &["semantic", "--output", "compile_commands.json"],
        format!("{event}\n").as_bytes(),
    )?;
    result.assert_success()?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: interception-events-format, cli-exit-codes
//
// An empty event stream yields an empty database and success, but it is
// almost always a plumbing mistake (a producer that emitted nothing, or a
// forgotten redirect), so a stderr notice must say the stream was empty.
#[test]
fn semantic_empty_stdin_succeeds_with_warning() -> Result<()> {
    let env = TestEnvironment::new("semantic_empty_stdin")?;

    let result = env.run_bear_with_stdin(&["semantic", "--output", "compile_commands.json"], b"")?;
    result.assert_success()?;

    let stderr = result.stderr();
    assert!(stderr.contains("no events"), "stderr must warn about the empty event stream: {stderr}");

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

// Requirements: recognition-cpp20-modules
//
// GCC's transitional module flag `-fmodules-ts` must not break recognition
// of the module-interface source (`mod.cppm`).
#[test]
fn gcc_modules_ts_flag_yields_single_entry_for_module_interface() -> Result<()> {
    let env = TestEnvironment::new("gcc_modules_ts_flag")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": "g++",
        "arguments": ["g++", "-std=c++20", "-fmodules-ts", "-c", "mod.cppm"],
        "working_dir": temp_dir,
        "environment": {}
    });

    env.create_source_files(&[("events.json", &event.to_string()), ("mod.cppm", "export module mod;\n")])?;

    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;
    db.assert_contains(&compilation_entry!(
        file: "mod.cppm".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            "g++".to_string(),
            "-std=c++20".to_string(),
            "-fmodules-ts".to_string(),
            "-c".to_string(),
            "mod.cppm".to_string(),
        ]
    ))?;

    Ok(())
}

// Requirements: interception-events-format
//
// A malformed line in the middle of the event stream must not drop every
// valid line that follows it: both the line before and the line after the
// malformed one must still become compilation-database entries, and the
// malformed line is reported with its physical line number on stderr at
// the default log level (no `RUST_LOG` opt-in required).
#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_input_mixed_valid_and_malformed_lines_keeps_both_valid_entries() -> Result<()> {
    let env = TestEnvironment::new("semantic_mixed_valid_malformed")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let event2 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "other.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    // Line 2 is deliberately not valid JSON at all.
    let events_content = format!("{event1}\nthis is not json\n{event2}\n");

    env.create_source_files(&[
        ("test.c", "int main() { return 0; }"),
        ("other.c", "int main() { return 0; }"),
    ])?;

    let result = env.run_bear_with_stdin_default_log(
        &["semantic", "--input", "-", "--output", "compile_commands.json"],
        events_content.as_bytes(),
    )?;
    result.assert_success()?;

    let stderr = result.stderr();
    assert!(
        stderr.contains("line 2"),
        "stderr must cite the malformed line's physical line number: {stderr}"
    );

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "test.c".to_string()]
    ))?;
    db.assert_contains(&compilation_entry!(
        file: "other.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![COMPILER_C_PATH.to_string(), "-c".to_string(), "other.c".to_string()]
    ))?;

    Ok(())
}

// Requirements: interception-events-format, cli-exit-codes
//
// When every line of a non-empty event stream is rejected, the run must
// not silently succeed with an empty compilation database: it exits
// non-zero so the failure is visible.
#[test]
fn semantic_input_all_malformed_lines_exits_non_zero() -> Result<()> {
    let env = TestEnvironment::new("semantic_all_malformed")?;

    let events_content = "not json\n{\"executable\": 42}\nalso not json\n";

    let result = env.run_bear_with_stdin(
        &["semantic", "--input", "-", "--output", "compile_commands.json"],
        events_content.as_bytes(),
    )?;
    result.assert_failure()?;

    Ok(())
}

// Requirements: output-json-compilation-database
//
// `bear semantic` runs no build, so unlike the combined/intercept modes its
// stdout is a safe, clean channel: `--output -` must stream the JSON
// compilation database to standard output, rather than literally creating a
// file named `-`. The stdout database must be identical to the one produced
// by the same input written to a file (same dedup/header/source-filter/
// format decorators, only the sink differs).
#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_output_stdout_matches_file_output() -> Result<()> {
    let env = TestEnvironment::new("semantic_output_stdout")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event1 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let event2 = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "other.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let events_content = format!("{event1}\n{event2}\n");

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "int main() { return 0; }"),
        ("other.c", "int main() { return 0; }"),
    ])?;

    // Baseline: the same event input, written to a file.
    env.run_bear_success(&["semantic", "--input", "events.json", "--output", "from_file.json"])?;
    let from_file = env.load_compilation_database("from_file.json")?;
    from_file.assert_count(2)?;

    // Under test: identical input, output streamed to stdout via `-`.
    let stdout_result = env.run_bear_success(&["semantic", "--input", "events.json", "--output", "-"])?;
    let stdout_entries: Vec<Value> = serde_json::from_str(&stdout_result.stdout())
        .context("stdout must be a valid JSON compilation database")?;
    assert_eq!(stdout_entries.len(), 2, "stdout database must contain 2 entries: {stdout_entries:?}");

    let mut file_entries: Vec<String> = from_file.entries().iter().map(|entry| entry.to_string()).collect();
    let mut stdout_strings: Vec<String> = stdout_entries.iter().map(|entry| entry.to_string()).collect();
    file_entries.sort();
    stdout_strings.sort();

    assert_eq!(
        stdout_strings, file_entries,
        "semantic --output - must produce the same compilation database as semantic --output <file>"
    );

    assert!(!env.file_exists("-"), "must not create a file literally named `-`");

    Ok(())
}

// Requirements: output-duplicate-detection
//
// Duplicate filtering must still run when the pipeline streams to stdout:
// two identical compiles collapse to one entry, proving the dedup decorator
// (and not just the base writer) is present in the stdout pipeline branch.
#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_output_stdout_deduplicates_entries() -> Result<()> {
    let env = TestEnvironment::new("semantic_output_stdout_dedup")?;
    let temp_dir = env.test_dir().to_str().unwrap();

    let event = json!({
        "executable": COMPILER_C_PATH,
        "arguments": [COMPILER_C_PATH, "-c", "test.c"],
        "working_dir": temp_dir,
        "environment": {}
    });
    let events_content = format!("{event}\n{event}\n");

    env.create_source_files(&[("events.json", &events_content), ("test.c", "int main() { return 0; }")])?;

    let result = env.run_bear_success(&["semantic", "--input", "events.json", "--output", "-"])?;
    let entries: Vec<Value> =
        serde_json::from_str(&result.stdout()).context("stdout must be a valid JSON compilation database")?;

    assert_eq!(entries.len(), 1, "duplicate compiles must collapse to one stdout entry: {entries:?}");

    Ok(())
}

// Requirements: output-append
//
// Appending means reading back the existing output before writing, which is
// impossible for a stream: `--append` combined with `--output -` must be
// rejected up front with a clear message, rather than attempting (and
// failing confusingly) to open `-` as a file.
#[test]
fn semantic_output_stdout_rejects_append() -> Result<()> {
    let env = TestEnvironment::new("semantic_output_stdout_append")?;

    env.create_source_files(&[("events.json", "")])?;

    let result = env.run_bear(&["semantic", "--input", "events.json", "--output", "-", "--append"])?;
    result.assert_failure()?;

    let stderr = result.stderr();
    assert!(
        stderr.contains("cannot append to standard output"),
        "stderr must explain why append + stdout output is rejected: {stderr}"
    );
    assert!(!env.file_exists("-"), "must not create a file literally named `-`");

    Ok(())
}

// Requirements: recognition-compiler-names
//
// `--print-compilers` is a pure static dump of the built-in recognition
// tables: dispatch happens before any input stream is opened, so the
// command must exit promptly even with stdin left open and undrained --
// unlike every other `semantic` invocation, which reads an event stream.
// Spawned directly (rather than through `run_bear`, whose `Command::output`
// already closes stdin) so an accidental stdin read on this path would show
// up here as a hang, not as a false pass.
#[test]
fn semantic_print_compilers_does_not_block_on_stdin() -> Result<()> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let env = TestEnvironment::new("semantic_print_compilers")?;

    let mut child = Command::new(env.bear_path())
        .current_dir(env.test_dir())
        .args(["semantic", "--print-compilers"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Keep the write end of stdin open for the whole wait -- never written
    // to, never dropped/closed. If the dispatcher read from it, this test
    // would hang instead of the process exiting on its own.
    let _stdin = child.stdin.take().context("stdin is piped")?;

    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!(
                "bear semantic --print-compilers did not exit within 5s; it must not block on stdin"
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    assert!(status.success(), "bear semantic --print-compilers must exit successfully");

    let mut stdout = String::new();
    child.stdout.take().context("stdout is piped")?.read_to_string(&mut stdout)?;

    assert!(stdout.contains("recognizes the following compilers"), "missing banner in stdout: {stdout}");
    assert!(stdout.contains("GCC"), "missing GCC entry in stdout: {stdout}");
    assert!(stdout.contains("as gcc"), "missing gcc alias in stdout: {stdout}");

    Ok(())
}

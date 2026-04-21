use crate::fixtures::infrastructure::compilation_entry;
use crate::fixtures::*;
use anyhow::Result;
use serde_json::json;

#[test]
#[cfg(has_executable_compiler_c)]
fn basic_semantic_conversion() -> Result<()> {
    let env = TestEnvironment::new("basic_semantic")?;

    let temp_dir = env.test_dir().to_str().unwrap();

    // Create sample events file with compilation events using new format
    // Use proper JSON serialization to handle Windows paths with backslashes

    let event1 = json!({
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event2 = json!({
        "pid": 12346,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test1.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event2 = json!({
        "pid": 12346,
        "execution": {
            "executable": COMPILER_CXX_PATH,
            "arguments": [COMPILER_CXX_PATH, "-c", "test2.cpp"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event3 = json!({
        "pid": 12347,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test3.c", "-o", "test3.o"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "-Wall", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "./src/main.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-DWRAPPER_FLAG", "-c", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-fplugin=libexample.so", "-c", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event2 = json!({
        "pid": 12346,
        "execution": {
            "executable": LS_PATH,
            "arguments": [LS_PATH, "-la"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event3 = json!({
        "pid": 12347,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test2.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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

#[test]
fn semantic_malformed_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_malformed")?;

    env.create_source_files(&[(
        "events.json",
        r#"{"invalid": "json"
{"pid": "not_a_number", "execution": {}}
{malformed json"#,
    )])?;

    // Bear should handle malformed events gracefully
    let _output =
        env.run_bear_success(&["semantic", "--input", "events.json", "--output", "compile_commands.json"])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

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
        "pid": 12345,
        "execution": {
            "executable": ECHO_PATH,
            "arguments": ["hello"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event2 = json!({
        "pid": 12346,
        "execution": {
            "executable": MKDIR_PATH,
            "arguments": ["-p", "build"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });

    let event3 = json!({
        "pid": 12347,
        "execution": {
            "executable": RM_PATH,
            "arguments": ["-f", "temp.txt"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 12345,
        "execution": {
            "executable": COMPILER_C_PATH,
            "arguments": [COMPILER_C_PATH, "-c", "test.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 1,
        "execution": {
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
        }
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
        "pid": 1,
        "execution": {
            "executable": cl,
            "arguments": [cl, "/Wv", "/c", "bare.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
    });
    let event_with_version = json!({
        "pid": 2,
        "execution": {
            "executable": cl,
            "arguments": [cl, "/Wv:17", "/c", "versioned.c"],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 1,
        "execution": {
            "executable": cl,
            "arguments": [
                cl,
                "/w14100", "/w24101", "/w34102", "/w44103",
                "/wd4995", "/we4996", "/wo4819",
                "/c", "test.c",
            ],
            "working_dir": temp_dir,
            "environment": {}
        }
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
        "pid": 1,
        "execution": {
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
        }
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

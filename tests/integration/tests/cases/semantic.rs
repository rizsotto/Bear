use crate::fixtures::infrastructure::compilation_entry;
use crate::fixtures::*;
use anyhow::{Context, Result};
use serde_json::json;

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
{}
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

// Requirements: recognition-mpi-wrappers
//
// An `mpicc -c hello.c` execution yields one entry, with the wrapper itself
// (not the underlying compiler) as the recorded compiler.
#[test]
fn mpi_wrapper_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("mpi_wrapper_execution", &["mpicc", "-c", "hello.c"])
}

// Requirements: recognition-mpi-wrappers
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

// Requirements: recognition-mpi-wrappers
//
// MPICH's compiler-override flag `-cc=gcc` must survive as a single token
// (not be split, not be expanded) and must not swallow the source file that
// follows it.
#[test]
fn mpi_wrapper_compiler_override_flag_is_retained() -> Result<()> {
    assert_driver_yields_single_entry("mpi_wrapper_compiler_override", &["mpicc", "-cc=gcc", "-c", "hello.c"])
}

// Requirements: recognition-mpi-wrappers
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

// Requirements: recognition-cray-compilers
//
// A `craycc -c hello.c` execution yields one entry, using the CCE C/C++
// compiler name directly (the same shape as the existing Cray Fortran
// support for `crayftn`/`ftn`).
#[test]
fn cray_cc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("cray_cc_execution", &["craycc", "-c", "hello.c"])
}

// Requirements: recognition-amd-compilers
//
// A `hipcc -c hello.c` execution yields one entry, using the ROCm HIP
// compiler driver name directly (parsed with Clang flag semantics).
#[test]
fn hipcc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("hipcc_execution", &["hipcc", "-c", "hello.c"])
}

// Requirements: recognition-embedded-toolchains
//
// A `qcc -c hello.c` execution yields one entry, using the QNX driver name
// directly (parsed with GCC flag semantics -- QNX 8 is GCC-backed).
#[test]
fn qnx_qcc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("qnx_qcc_execution", &["qcc", "-c", "hello.c"])
}

// Requirements: recognition-embedded-toolchains
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

// Requirements: recognition-embedded-toolchains
//
// A `tiarmclang -c hello.c` execution yields one entry (TI's clang-based
// driver, parsed with Clang flag semantics).
#[test]
fn ti_tiarmclang_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("ti_tiarmclang_execution", &["tiarmclang", "-c", "hello.c"])
}

// Requirements: recognition-embedded-toolchains
//
// An `xc8-cc -c hello.c` execution yields one entry (Microchip's gcc-styled
// XC8 driver, parsed with GCC flag semantics).
#[test]
fn microchip_xc8_cc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("microchip_xc8_cc_execution", &["xc8-cc", "-c", "hello.c"])
}

// Requirements: recognition-embedded-toolchains
//
// An `emcc -c hello.c` execution yields one entry, using the Emscripten
// driver name directly (parsed with Clang flag semantics).
#[test]
fn emscripten_emcc_execution_yields_single_entry() -> Result<()> {
    assert_driver_yields_single_entry("emscripten_emcc_execution", &["emcc", "-c", "hello.c"])
}

// Requirements: recognition-embedded-toolchains
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

// Requirements: recognition-assemblers
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

// Requirements: recognition-assemblers
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

// Requirements: recognition-assemblers
//
// Direct assembly through a driver (`gcc -c foo.s`) is already recorded
// today via the driver's own entry -- this is the actual fix for the
// compile-then-assemble class of bug reported in issue #146, and it needs
// no new recognition code (the standalone-assembler recognizer added by
// this requirement is not involved). Locking this in guards against a
// future regression that would make it invisible again.
//
// This does not use `assert_driver_yields_single_entry_for_source`: unlike
// the synthetic embedded-toolchain names used elsewhere in this file,
// `gcc` is a real, `PATH`-resolvable executable on any host that can build
// Bear (its own host requirements list a `cc` toolchain), so
// `ExecutableResolver` rewrites the recorded compiler token to gcc's
// absolute path. The assertion below only pins the parts this test cares
// about: one entry, for `foo.s`, invoked with `-c foo.s`, as `gcc`
// (whichever path it resolved to).
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

// Requirements: recognition-swift-compiler
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

// Requirements: recognition-swift-compiler
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

    // PATH shield: without it, a host that really ships swiftc (macOS
    // runners) resolves the bare name to its absolute path and the
    // literal-argv assertions below fail on that platform only.
    let shield = env.test_dir().join("path-shield");
    std::fs::create_dir_all(&shield)?;

    let event = json!({
        "executable": "swiftc",
        "arguments": ["swiftc", "-module-name", "App", "-emit-object", "a.swift", "b.swift"],
        "working_dir": temp_dir,
        "environment": { "PATH": shield.to_str().unwrap() }
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

// Requirements: recognition-swift-compiler
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

// Requirements: recognition-swift-compiler
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

// Requirements: recognition-swift-compiler
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

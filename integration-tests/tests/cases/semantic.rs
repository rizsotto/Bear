use crate::fixtures::infrastructure::{compilation_entry, filename_of};
use crate::fixtures::*;
use anyhow::Result;
use serde_json::json;

#[test]
#[cfg(has_executable_compiler_c)]
fn basic_semantic_conversion() -> Result<()> {
    let env = TestEnvironment::new("basic_semantic")?;

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "int main() { return 0; }"),
    ])?;

    // Run semantic to convert events to compilation database
    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    // Verify compilation database was created
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the compilation entry matches expected format
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![filename_of(COMPILER_C_PATH), "-c".to_string(), "test.c".to_string()]
    ))?;

    Ok(())
}

#[test]
#[cfg(all(has_executable_compiler_c, has_executable_compiler_cxx))]
fn semantic_multiple_entries() -> Result<()> {
    let env = TestEnvironment::new("semantic_multiple")?;

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(3)?;

    // Verify all compilation entries
    db.assert_contains(&compilation_entry!(
        file: "test1.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![filename_of(COMPILER_C_PATH), "-c".to_string(), "test1.c".to_string()]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test2.cpp".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![filename_of(COMPILER_CXX_PATH), "-c".to_string(), "test2.cpp".to_string()]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test3.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![filename_of(COMPILER_C_PATH), "-c".to_string(), "test3.c".to_string(), "-o".to_string(), "test3.o".to_string()]
    ))?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_format_conversion() -> Result<()> {
    let env = TestEnvironment::new("semantic_format")?;

    let temp_dir = env.temp_dir().to_str().unwrap();

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
        (
            "test.c",
            "#include <stdio.h>\nint main() { printf(\"Hello\\n\"); return 0; }",
        ),
    ])?;

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the compilation entry preserves compiler flags
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            filename_of(COMPILER_C_PATH),
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

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    env.create_source_files(&[
        ("events.json", &events_content),
        ("src/main.c", "int main() { return 0; }"),
    ])?;

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify relative paths are handled correctly
    db.assert_contains(&compilation_entry!(
        file: "./src/main.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            filename_of(COMPILER_C_PATH),
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

    let temp_dir = env.temp_dir().to_str().unwrap();

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
        (
            "test.c",
            "#ifdef WRAPPER_FLAG\nint main() { return 0; }\n#endif",
        ),
    ])?;

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify wrapper flags are preserved
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            filename_of(COMPILER_C_PATH),
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

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "int main() { return 0; }"),
    ])?;

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify plugin flags are preserved
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            filename_of(COMPILER_C_PATH),
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

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Should only contain the 2 compilation commands, not the ls command
    db.assert_count(2)?;

    // Verify only compilation entries are included
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![filename_of(COMPILER_C_PATH), "-c".to_string(), "test.c".to_string()]
    ))?;

    db.assert_contains(&compilation_entry!(
        file: "test2.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![filename_of(COMPILER_C_PATH), "-c".to_string(), "test2.c".to_string()]
    ))?;

    Ok(())
}

#[test]
fn semantic_empty_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_empty")?;

    env.create_source_files(&[("events.json", "")])?;

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

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
    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(0)?;

    Ok(())
}

#[test]
#[cfg(all(has_executable_echo, has_executable_mkdir, has_executable_rm))]
fn semantic_non_compilation_events() -> Result<()> {
    let env = TestEnvironment::new("semantic_non_compilation")?;

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    // Should contain no entries since none are compilation commands
    db.assert_count(0)?;

    Ok(())
}

#[test]
#[cfg(has_executable_compiler_c)]
fn semantic_output_format() -> Result<()> {
    let env = TestEnvironment::new("semantic_output_format")?;

    let temp_dir = env.temp_dir().to_str().unwrap();

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

    env.create_source_files(&[
        ("events.json", &events_content),
        ("test.c", "int main() { return 0; }"),
    ])?;

    let _output = env.run_bear_success(&[
        "semantic",
        "--input",
        "events.json",
        "--output",
        "compile_commands.json",
    ])?;

    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(1)?;

    // Verify the entry has the expected format with defines
    db.assert_contains(&compilation_entry!(
        file: "test.c".to_string(),
        directory: temp_dir.to_string(),
        arguments: vec![
            filename_of(COMPILER_C_PATH),
            "-c".to_string(),
            "test.c".to_string()
        ]
    ))?;

    Ok(())
}

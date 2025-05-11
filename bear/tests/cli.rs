// bear/tests/cli.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::error::Error;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_bear_help() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("bear")?;
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: bear"));
    Ok(())
}

#[test]
#[cfg(target_os = "linux")]
#[cfg(has_executable_echo)]
fn test_wrapper_basic() -> Result<(), Box<dyn Error>> {
    let work_dir = tempdir()?;

    let mut cmd = Command::cargo_bin("bear")?;
    cmd.args(["--", "echo", "hello"]);
    cmd.current_dir(work_dir.path());

    println!("Running command: {:?}", cmd);

    // Add assertions based on what bear/wrapper should do
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello")); // Adjust assertion as needed

    work_dir.close()?; // Clean up the temporary directory
    Ok(())
}

// Add more tests for different scenarios, arguments, and interactions
// between 'bear' and 'wrapper'.

#[test]
#[cfg(target_os = "linux")]
#[cfg(has_executable_make)] // Only compile this test if 'make' was found by build.rs
fn test_with_make() -> Result<(), Box<dyn Error>> {
    let make_path = env!("MAKE_PATH"); // Get path from env var set by build.rs
    println!("Make found at: {}", make_path);

    let mut cmd = Command::cargo_bin("bear")?;
    let work_dir = tempdir()?;

    // Setup: Create a dummy Makefile or project structure
    std::fs::write(
        work_dir.path().join("Makefile"),
        "all:\n\techo \"Running make\"\n",
    )?;

    cmd.current_dir(work_dir.path());
    // Use the detected make path if needed, or just run 'make' if it's in PATH
    cmd.args(["--", make_path, "all"]); // Example: run make through bear

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Running make"));

    // Check for compile_commands.json, etc.
    // assert!(work_dir.path().join("compile_commands.json").exists());

    work_dir.close()?;
    Ok(())
}

// Example test requiring 'cc'
#[test]
#[cfg(has_executable_compiler_c)]
fn test_with_cc() -> Result<(), Box<dyn Error>> {
    let cc_path = env!("COMPILER_C_PATH");
    println!("C Compiler found at: {}", cc_path);
    // ... test logic using cc_path ...
    assert!(Path::new(cc_path).exists()); // Basic check
    Ok(())
}

// Example test requiring 'c++'
#[test]
#[cfg(has_executable_compiler_cxx)]
fn test_with_cplusplus() -> Result<(), Box<dyn Error>> {
    let cxx_path = env!("COMPILER_CXX_PATH");
    println!("C++ Compiler found at: {}", cxx_path);
    // ... test logic using cxx_path ...
    assert!(Path::new(cxx_path).exists()); // Basic check
    Ok(())
}

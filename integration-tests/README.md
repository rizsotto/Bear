# Bear Integration Tests

This directory contains the comprehensive integration test suite for Bear, a tool that generates compilation databases for clang tooling. The integration tests verify Bear's functionality across different platforms, build systems, and usage scenarios.

## Directory Structure

```
integration-tests/
├── README.md                 # This file - overview and usage guide
├── Cargo.toml                # Test package configuration and dependencies
├── build.rs                  # Build script for platform capability detection
└── tests/
    ├── integration.rs
    ├── fixtures/             # Test infrastructure and utilities
    │   ├── mod.rs
    │   ├── constants.rs      # Test constants and configuration
    │   ├── external_dependencies.rs # External tool availability tests
    │   └── infrastructure.rs # Core test infrastructure (TestEnvironment, etc.)
    └── cases/                # Actual integration test implementations
        ├── mod.rs
        └── *.rs              # Actual integration test case implementation
```

## Running Integration Tests

### Prerequisites
1. **Enable Feature Flag**: Integration tests require the `allow-integration-tests` feature
2. **Build Dependencies**: Bear, intercept-preload, and intercept-wrapper must be built

### Basic Test Execution
```bash
# Build all dependencies first
cargo build --features allow-integration-tests 

# Run all integration tests
cargo test --features allow-integration-tests

# Run specific test file
cargo test --features allow-integration-tests compilation_output

# Run specific test function
cargo test --features allow-integration-tests test_basic_c_compilation
```

### Verbose Debugging
For debugging failing tests, use the verbose output system:

```bash
# Automatic verbose output on test failure
BEAR_TEST_VERBOSE=1 cargo test --features allow-integration-tests

# Preserve test directories for manual inspection
BEAR_TEST_PRESERVE_FAILURES=1 cargo test --features allow-integration-tests

# Combine both for thorough debugging
BEAR_TEST_VERBOSE=1 BEAR_TEST_PRESERVE_FAILURES=1 cargo test --features allow-integration-tests
```

## Writing New Tests

### Test Infrastructure Overview

The test infrastructure provides several key components:

1. **TestEnvironment**: Manages temporary directories, creates test files, and runs Bear
2. **BearOutput**: Captures and displays Bear's stdout/stderr with verbose support
3. **CompilationDatabase**: Validates compilation database contents with detailed assertions
4. **bear_test! macro**: Simplifies test creation with built-in verbose support

### Basic Test Pattern

```rust
use crate::fixtures::*;
use anyhow::Result;

#[test]
fn test_my_new_feature() -> Result<()> {
    // Create test environment
    let env = TestEnvironment::new("my_test")?;
    
    // Create source files
    env.create_source_files(&[
        ("main.c", "int main() { return 0; }"),
        ("lib.c", "void func() {}"),
    ])?;
    
    // Create build script
    env.create_build_script("gcc -c main.c lib.c")?;
    
    // Run Bear
    let output = env.run_bear(&[
        "--output", "compile_commands.json",
        "--", "sh", "build.sh"
    ])?;
    
    // Load and validate compilation database
    let db = env.load_compilation_database("compile_commands.json")?;
    db.assert_count(2)?;
    db.assert_contains_file("main.c")?;
    db.assert_contains_file("lib.c")?;
    
    Ok(())
}
```

### Using the bear_test! Macro

For simpler tests, use the `bear_test!` macro:

```rust
bear_test!(test_simple_compilation, |env| {
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
    env.create_build_script("gcc -c test.c")?;
    
    let output = env.run_bear(&["--output", "db.json", "--", "sh", "build.sh"])?;
    let db = env.load_compilation_database("db.json")?;
    
    db.assert_count(1)?;
    db.assert_contains_file("test.c")?;
    Ok(())
});
```

### Platform-Specific Tests

Use conditional compilation for platform-specific functionality:

```rust
#[cfg(target_os = "linux")]
#[test]
fn test_preload_specific_feature() -> Result<()> {
    // Test that only runs on Linux where preload is available
    let env = TestEnvironment::new("preload_test")?;
    // ... test implementation
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
#[test]
fn test_wrapper_specific_feature() -> Result<()> {
    // Test for platforms using wrapper-based interception
    let env = TestEnvironment::new("wrapper_test")?;
    // ... test implementation
    Ok(())
}
```

### Debugging Failed Tests

When tests fail, the verbose output system provides detailed information:

1. **Bear's complete execution logs** showing interception, semantic analysis, and output generation
2. **Assertion failure details** with expected vs. actual compilation database contents
3. **Pretty-printed JSON** of actual compilation entries

Use `BEAR_TEST_VERBOSE=1` to automatically see this information for any failing test.

### Test Organization Guidelines

1. **Group by Functionality**: Tests are organized by the Bear feature they test (interception, semantics, output, etc.)
2. **Platform Awareness**: Use appropriate conditional compilation for platform-specific tests
3. **Descriptive Names**: Test names should clearly indicate what functionality is being tested
4. **Error Handling**: Always use `Result<()>` return type and the `?` operator for error propagation
5. **Cleanup**: The test infrastructure automatically handles temporary directory cleanup

### Adding External Tool Dependencies

If your test requires external tools (compilers, build systems), check for availability:

1. Add capability checks to `build.rs` if needed
2. Use conditional compilation based on tool availability: `#[cfg(has_executable_make)]`
3. Follow existing patterns in `external_dependencies.rs`

### Best Practices

1. **Test One Thing**: Each test should focus on a specific aspect of Bear's functionality
2. **Use Realistic Scenarios**: Create tests that mirror real-world usage patterns
3. **Provide Good Error Messages**: Use descriptive assertion messages for easier debugging
4. **Test Edge Cases**: Include tests for error conditions and unusual inputs
5. **Document Complex Tests**: Add comments explaining the purpose and setup of complex test scenarios

## Integration with CI/CD

The integration tests are designed to run in CI environments:

- **Feature Flag**: Tests only run when explicitly enabled with `allow-integration-tests`
- **Platform Detection**: Build script detects available tools and sets appropriate cfg flags
- **Parallel Execution**: Tests use isolated temporary directories for safe parallel execution
- **Error Reporting**: Verbose output provides detailed debugging information for CI failures

## Common Test Patterns

### Testing Compilation Database Content
```rust
let db = env.load_compilation_database("compile_commands.json")?;
db.assert_count(2)?;
db.assert_contains_file("main.c")?;
db.assert_entry_has_argument("main.c", "-Wall")?;
```

### Testing Bear Configuration Options
```rust
let output = env.run_bear(&[
    "--config", "config.yml",
    "--output", "custom.json",
    "--", "make"
])?;
```

### Testing Error Conditions
```rust
let output = env.run_bear(&["--invalid-option"])?;
assert!(output.status().is_some());
assert_eq!(output.status().unwrap(), 1);
```

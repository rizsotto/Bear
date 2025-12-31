# Integration Test Verbose Output Guide

This document explains how to use the verbose output functionality in Bear's integration tests to debug failing tests.

## Problem

When integration tests fail, especially assertion failures in compilation database validation, it can be difficult to debug because you can't see:
- What Bear actually output (stdout/stderr)
- What the compilation database actually contains
- Why assertions are failing

## Solution

The integration test infrastructure now supports verbose output that shows detailed information when tests fail.

## Usage Methods

### Method 1: Environment Variable (Automatic)

Set `BEAR_TEST_VERBOSE=1` to automatically show verbose output when ANY test fails:

```bash
BEAR_TEST_VERBOSE=1 cargo test --features allow-integration-tests
```

This will automatically display:
- Bear's stdout and stderr output
- Compilation database debug information (expected vs actual)
- Pretty-printed JSON of actual compilation entries

### Method 2: Explicit Verbose Environment

Create a test environment with verbose mode enabled:

```rust
#[test]
fn my_test() -> Result<()> {
    let env = TestEnvironment::new_with_verbose("my_test", true)?;
    // ... rest of test
}
```

### Method 3: Manual Output Display

Show output explicitly in your test:

```rust
#[test]
fn my_test() -> Result<()> {
    let env = TestEnvironment::new("my_test")?;
    let output = env.run_bear(&["--output", "db.json", "--", "gcc", "-c", "test.c"])?;

    // Show output only if verbose mode is enabled
    output.show_verbose_if_enabled();

    // Force show output regardless of verbose setting
    output.force_show_verbose();

    // Show last bear output from environment
    env.show_last_bear_output();

    // ... assertions
}
```

### Method 4: Enhanced bear_test! Macro

The bear_test! macro has been enhanced to support verbose mode:

```rust
// Standard macro usage (respects BEAR_TEST_VERBOSE environment variable)
bear_test!(my_test, |env| {
    let output = env.run_bear(&["--output", "db.json", "--", "make"])?;
    let db = env.load_compilation_database("db.json")?;
    db.assert_count(2)?; // Will show verbose info if this fails
    Ok(())
});
```

## What You'll See

### Combined Bear Output and Assertion Debug Info
When an assertion fails in verbose mode, you'll see both the Bear command output AND the assertion details:

```
=== Bear Command Output ===
Bear stdout:
  (empty)
Bear stderr:
  [2025-12-31T13:16:05Z DEBUG bear] bear v4.0.0
  [2025-12-31T13:16:05Z DEBUG bear] Running on... unix/linux x86_64
  [2025-12-31T13:16:05Z DEBUG bear] Application Context:
        Current Executable: /home/user/Code/Bear.rust.git/target/debug/bear
        Current Directory: /tmp/.tmpItPv3K
        Total Environment Variables: 101 entries
  [2025-12-31T13:16:05Z DEBUG bear] Arguments: Arguments { config: None, mode: Combined { ... } }
  [2025-12-31T13:16:05Z INFO  bear::intercept::environment] Build command to run: BuildCommand { arguments: ["/usr/bin/sh", "/tmp/.tmpItPv3K/build.sh"] }
  [2025-12-31T13:16:05Z DEBUG bear::semantic::interpreters::combinators] Recognizing execution: Execution { executable: "/usr/lib64/ccache/gcc", arguments: ["gcc", "-c", "-o", "valid.o", "valid.c"], working_dir: "/tmp/.tmpItPv3K", ... }
  ... (detailed Bear debug logs)
Bear exit code: Some(2)
=== End Bear Output ===

=== Compilation Database Debug Info ===
Expected 2 entries, but found 3
Actual entries:
  Entry 0: {
    "arguments": ["gcc", "-c", "-o", "valid.o", "valid.c"],
    "directory": "/tmp/.tmpItPv3K",
    "file": "valid.c",
    "output": "valid.o"
  }
  Entry 1: {
    "arguments": ["gcc", "-c", "-o", "invalid.o", "invalid.c"],
    "directory": "/tmp/.tmpItPv3K",
    "file": "invalid.c",
    "output": "invalid.o"
  }
  Entry 2: {
    "arguments": ["/usr/bin/gcc", "-c", "-fdiagnostics-color", "-o", "invalid.o", "invalid.c"],
    "directory": "/tmp/.tmpItPv3K",
    "file": "invalid.c",
    "output": "invalid.o"
  }
=== End Debug Info ===
```

### Automatic Display on Test Failure
When `BEAR_TEST_VERBOSE=1` is set and a test fails (panics), the infrastructure automatically shows:
```
=== Bear Verbose Output (Test: my_failing_test) ===
Bear stdout:
  ...
Bear stderr:
  ...
Bear exit code: Some(2)
=== End Bear Output ===
```

## Environment Variables

- `BEAR_TEST_VERBOSE=1`: Enable automatic verbose output on test failure
- `BEAR_TEST_PRESERVE_FAILURES=1`: Preserve test directories on failure (existing functionality)

## Examples

### Debug a failing assertion:
```bash
BEAR_TEST_VERBOSE=1 cargo test --features allow-integration-tests failing_test_name
```

### Show output for a specific test:
```rust
#[test]
fn debug_compilation_count() -> Result<()> {
    let env = TestEnvironment::new_with_verbose("debug_test", true)?;
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;

    let output = env.run_bear(&["--output", "db.json", "--", "gcc", "-c", "test.c"])?;
    let db = env.load_compilation_database("db.json")?;

    // This will show verbose debug info if the count is wrong
    db.assert_count(1)?;
    Ok(())
}
```

### Force show output for debugging:
```rust
#[test]
fn always_show_output() -> Result<()> {
    let env = TestEnvironment::new("test")?;
    let output = env.run_bear(&["--output", "db.json", "--", "make"])?;

    // Always show the bear output for this test
    output.force_show_verbose();

    Ok(())
}
```

## Tips

1. **Use environment variable for general debugging**: `BEAR_TEST_VERBOSE=1` is the easiest way to debug failing tests - you'll see both Bear's detailed execution logs AND assertion failure details
2. **Use explicit verbose for specific tests**: When developing new tests, use `new_with_verbose(name)`
3. **Force show output for complex scenarios**: Use `force_show_verbose()` when you need to see output regardless of test outcome
4. **Combine with test preservation**: Use both `BEAR_TEST_VERBOSE=1` and `BEAR_TEST_PRESERVE_FAILURES=1` for thorough debugging
5. **Read Bear logs carefully**: The verbose output shows Bear's complete execution flow, including which commands were intercepted, how they were classified, and what compilation entries were generated

## Implementation Details

The verbose infrastructure:
- Stores the last Bear output in each `TestEnvironment`
- Automatically displays Bear output AND assertion debug info when assertions fail in verbose mode
- Uses a panic handler in the `Drop` implementation to catch test failures
- Passes Bear output reference to `CompilationDatabase` for assertion failure reporting
- Shows Bear's complete execution logs including interception, semantic analysis, and compilation database generation
- Provides both automatic and manual control over output display
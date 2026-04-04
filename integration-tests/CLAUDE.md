## Integration test rules

Read `README.md` in this directory for full API documentation (TestEnvironment,
BearOutput, CompilationDatabase, bear_test! macro, platform-specific patterns).

## Before writing tests

- A debug build must exist: run `cargo build` first
- Check `build.rs` for platform capability detection flags
- Use conditional compilation for platform-specific tests: `#[cfg(has_executable_make)]`

## Test pattern

```rust
bear_test!(test_name, |env| {
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
    env.create_build_script("gcc -c test.c")?;

    let output = env.run_bear(&["--output", "db.json", "--", "sh", "build.sh"])?;
    let db = env.load_compilation_database("db.json")?;

    db.assert_count(1)?;
    db.assert_contains_file("test.c")?;
    Ok(())
});
```

## Naming convention

Tests that protect a specific requirement should reference it in the name:

```
test_req_<requirement_id>_<description>
```

Example: `test_req_output_001_json_format` protects requirement `output-001`.

This makes it possible to trace which requirements have test coverage.

## Debugging

```bash
BEAR_TEST_VERBOSE=1 cargo test                       # verbose on failure
BEAR_TEST_PRESERVE_FAILURES=1 cargo test             # keep temp dirs
BEAR_TEST_VERBOSE=1 BEAR_TEST_PRESERVE_FAILURES=1 cargo test  # both
```

## Regression protection role

Integration tests are the primary regression protection mechanism for Bear.

- Every implemented requirement should have at least one integration test
- Tests should reference the requirement they protect (via naming or comments)
- When a bug is fixed, add a test that reproduces the original failure
- Platform-specific behavior needs platform-specific tests with `#[cfg(...)]`

## Organization

- `tests/cases/` - test implementations grouped by feature area
- `tests/fixtures/` - test infrastructure (TestEnvironment, assertions, constants)
- `tests/integration.rs` - test entry point

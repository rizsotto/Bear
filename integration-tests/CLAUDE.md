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

## Linking tests to requirements

Tests that protect a requirement cite its ID with a `Requirements:` tag. This
is the sole source of truth for the test-to-requirement link; requirement files
do not list their tests.

Format:

```rust
// Requirements: output-json-compilation-database, output-append
#[test]
fn append_works_as_expected() -> Result<()> { ... }
```

Rules:

- Value is a comma-separated list of requirement IDs (filenames in
  `requirements/` without the `.md` extension).
- Place the tag on the line(s) directly above `#[test]` (or the test macro).
- If every test in a file covers the same requirement, a file-level
  `//! Requirements: <id>` near the top is sufficient. Test-level tags
  override file-level tags.
- A test may cite multiple requirements when it legitimately exercises more
  than one.

To find tests for a requirement:

```bash
grep -rn "Requirements:.*<requirement-id>" bear/ intercept-preload/ integration-tests/
```

See `requirements/CLAUDE.md` for the coverage-check script that verifies every
`implemented` requirement has at least one tagged test.

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

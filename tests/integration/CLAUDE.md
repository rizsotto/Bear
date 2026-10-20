## Integration test rules

The test infrastructure (TestEnvironment, BearOutput, CompilationDatabase, the
`bear_test!` macro) lives in `tests/fixtures/infrastructure/`; read those
sources for the full API and platform-specific patterns.

## Before writing tests

- A debug build must exist: run `cargo build` first. The tests run the
  driver, wrapper, and preload library that `build.rs` finds in the
  profile directory.
- Gate platform-specific tests on the `cfg` keys that `build.rs` emits,
  for example `#[cfg(has_executable_make)]`.
- `build.rs` probes a fixed list of host tools. To gate a test on a tool
  that is not in the list, add the probe there first.

## Test pattern

```rust
// Requirements: output-json-compilation-database
#[test]
#[cfg(all(has_executable_compiler_c, has_executable_shell))]
fn single_file_compilation() -> Result<()> {
    let env = TestEnvironment::new("single_file_compilation")?;
    env.create_source_files(&[("test.c", "int main() { return 0; }")])?;
    let build = format!("{} -c test.c", filename_of(COMPILER_C_PATH));
    let script = env.create_shell_script("build.sh", &build)?;

    let output = env.run_bear(&["--output", "db.json", "--", SHELL_PATH, script.to_str().unwrap()])?;
    output.assert_success()?;

    let db = env.load_compilation_database("db.json")?;
    db.assert_count(1)?;
    Ok(())
}
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

- Value is a comma-separated list of requirement IDs (see
  `docs/requirements/CLAUDE.md`).
- Place the tag on the line(s) directly above `#[test]` (or the test macro).
- If every test in a file covers the same requirement, a file-level
  `//! Requirements: <id>` near the top is sufficient. Test-level tags
  override file-level tags.
- A test may cite multiple requirements when it legitimately exercises more
  than one.

To find tests for a requirement:

```sh
grep -rn "Requirements:.*<requirement-id>" crates/ tests/
```

To verify every `implemented` requirement has at least one tagged test
(exits non-zero if any lacks coverage):

```sh
./scripts/check-requirements-coverage.sh
```

## Debugging

When a test panics, the fixture automatically dumps the last captured bear
stdout/stderr to the test binary's stderr. `run_bear` inherits `RUST_LOG`
when set; if unset it defaults to `info` (so warn/info/error log lines reach
the panic dump without the per-event debug spam from the preload library).

```sh
cargo test                                          # info-level dump on failure
RUST_LOG=debug cargo test                           # full per-event trace (recommended for local triage)
BEAR_TEST_PRESERVE_FAILURES=1 cargo test            # also keep temp dirs at /tmp/bear-test-<name>-<pid>
RUST_LOG=debug BEAR_TEST_PRESERVE_FAILURES=1 cargo test  # both
```

CI sets `RUST_LOG=debug` so failures on platforms that can't be reproduced
locally carry full diagnostic context without a re-run.

## Organization

- `tests/cases/` - test implementations grouped by feature area
- `tests/fixtures/` - test infrastructure (TestEnvironment, assertions, constants)
- `tests/integration.rs` - test entry point

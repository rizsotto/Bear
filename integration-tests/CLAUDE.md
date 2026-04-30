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

When a test panics, the fixture automatically dumps the last captured bear
stdout/stderr to the test binary's stderr. `run_bear` inherits `RUST_LOG`
when set; if unset it defaults to `info` (so warn/info/error log lines reach
the panic dump without the per-event debug spam from the preload library).

```bash
cargo test                                          # info-level dump on failure
RUST_LOG=debug cargo test                           # full per-event trace (recommended for local triage)
BEAR_TEST_PRESERVE_FAILURES=1 cargo test            # also keep temp dirs at /tmp/bear-test-<name>-<pid>
RUST_LOG=debug BEAR_TEST_PRESERVE_FAILURES=1 cargo test  # both
```

CI sets `RUST_LOG=debug` so failures on platforms that can't be reproduced
locally carry full diagnostic context without a re-run.

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

## Build script duties

`build.rs` does:

- Forwards `INTERCEPT_LIBDIR` and emits paths to the driver, wrapper,
  and preload artifacts as `cargo:rustc-env=` vars (consumed via
  `env!()` from the test fixtures).
- Replays `platform-checks` results via `emit_cfg()` /
  `emit_check_cfg()`.
- Probes a fixed list of host executables (see `build.rs`) via
  `which`, emitting `cargo:rustc-cfg=has_executable_<name>` for
  single tools and `has_executable_<group>` for grouped lookups
  (`shell`, `make`, `compiler_c`, `compiler_cxx`, `compiler_fortran`,
  `compiler_cuda`). Tests gate on these via `#[cfg(has_executable_*)]`.
- Searches well-known paths for a `ccache` masquerade directory,
  emitting `cargo:rustc-cfg=host_has_ccache_masquerade` when found.

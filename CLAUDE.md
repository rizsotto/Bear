## Pre-commit checks (mandatory)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Do not commit unless all three pass. Fix issues before committing.

## Build

```bash
cargo build --verbose          # debug
cargo build --release          # release (LTO, stripped)
```

Integration tests require a debug build to exist before running `cargo test`.

## Project overview

Bear generates JSON compilation databases (`compile_commands.json`) for Clang tooling.
It intercepts compiler invocations during a build and records them.

### Workspace crates

| Crate | Purpose |
|---|---|
| `bear` | Main driver, CLI, semantic analysis, output |
| `intercept-preload` | `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES` shared library |
| `platform-checks` | Build-time platform capability detection |
| `bear-codegen` | Code generator for compiler flag tables |
| `integration-tests` | End-to-end tests |

## Routing - read before modifying

Before modifying code in a subdirectory, read the `CLAUDE.md` in that directory first.
These files contain rules, context, and constraints specific to that area.

| When you are about to... | Read first |
|---|---|
| Modify CLI arguments or output format | `bear/CLAUDE.md` |
| Edit or add compiler interpreter YAML | `bear/interpreters/CLAUDE.md` |
| Edit the YAML-to-Rust code generator | `bear-codegen/CLAUDE.md` |
| Add or change a host capability probe | `platform-checks/CLAUDE.md` |
| Touch the preload interception library | `intercept-preload/CLAUDE.md` |
| Touch the shell-completions binary | `bear-completions/CLAUDE.md` |
| Write or modify integration tests | `integration-tests/CLAUDE.md` |
| Edit or regenerate the man page | `man/CLAUDE.md` |
| Add, modify, or review a requirement | `requirements/CLAUDE.md` |

Do not skip these reads. They contain constraints that prevent regressions.

## Architecture (data flow)

1. **Interception** - capture compiler invocations via `LD_PRELOAD` (Linux/macOS) or wrapper executable (other platforms)
2. **Semantic analysis** - filter non-compiler commands using interpreter YAML definitions
3. **Configuration** - apply user config for output formatting
4. **Output** - write `compile_commands.json`

## Build pipeline

The workspace builds in three layers before linking the user-facing binaries:

1. `bear-codegen` runs from `bear/build.rs` to emit interpreter tables.
   See `bear-codegen/CLAUDE.md`.
2. `platform-checks/build.rs` probes the host once for headers and
   symbols. Consumers replay the results via
   `platform_checks::emit_cfg()` / `emit_check_cfg()`. See
   `platform-checks/CLAUDE.md`.
3. `intercept-preload/build.rs` cc-compiles `src/c/shim.c` and emits
   cdylib link directives. See `intercept-preload/CLAUDE.md`.

`INTERCEPT_LIBDIR` is the one cross-cutting build-time env var
(relative path to the install location of `libexec.so` /
`libexec.dylib`; defaults to `lib`). Validated and forwarded by both
`bear/build.rs` and `integration-tests/build.rs`.

## Host requirements

- `cc` toolchain (gcc or clang).
- `lld` linker (Linux only). The ELF version script uses multiple
  version tags, which GNU ld does not support; the link step on
  Linux fails without lld. macOS uses the system linker.
- `ccache` (optional). When present and on PATH, the integration tests
  exercise a ccache-masquerade-aware test path.

## Decision protocol

For architectural changes or new features:

1. Check `requirements/` for existing requirement specs
2. Write a requirement spec if one does not exist (see `requirements/CLAUDE.md`)
3. Provide a Decision Log before writing code:
   - Proposed approach
   - Alternatives considered
   - Trade-offs (performance vs simplicity)
4. Await "GO" before implementation
5. Write integration tests that reference the requirement

## Code guidelines

- Follow Rust 2024 edition idioms
- Maintain existing code structure
- Write tests for new functionality
- Keep dependencies minimal
- No speculative features or over-engineering

## Output format

- ASCII only in all files (no em dashes, smart quotes, Unicode bullets)
- Use hyphens, straight quotes, three dots, asterisks/hyphens for bullets

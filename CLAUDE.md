## Pre-commit checks (mandatory)

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Do not commit unless all three pass. Fix issues before committing.

## Build

```sh
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
| `crates/bear` | Main driver, CLI, semantic analysis, output |
| `crates/bear-driver` | Driver/supervisor binary; orchestrates interception and output |
| `crates/intercept` | Shared/agent-side interception runtime (Execution, reporter, wire, env helpers) |
| `crates/intercept-supervisor` | Driver-side interception (supervise, TCP collector, build environment) |
| `crates/intercept-preload` | `LD_PRELOAD` / `DYLD_INSERT_LIBRARIES` shared library |
| `crates/bear-wrapper` | Compiler wrapper binary; reports intercepted executions |
| `crates/bear-completions` | Shell-completion script generator |
| `build-support/platform-checks` | Build-time platform capability detection |
| `build-support/compilers-codegen` | Code generator for compiler flag tables |
| `tests/tools` | Shared test tooling; library + `cdb-compare` binary (package `bear-test-tools`) |
| `tests/integration` | End-to-end tests (package `integration-tests`) |

## Routing - read before modifying

Before modifying code in a subdirectory, read the `CLAUDE.md` in that directory first.
These files contain rules, context, and constraints specific to that area.

| When you are about to... | Read first |
|---|---|
| Modify CLI arguments or output format | `crates/bear/CLAUDE.md` |
| Edit or add compiler interpreter YAML | `crates/bear/compilers/CLAUDE.md` |
| Edit the YAML-to-Rust code generator | `build-support/compilers-codegen/CLAUDE.md` |
| Add or change a host capability probe | `build-support/platform-checks/CLAUDE.md` |
| Touch the preload interception library | `crates/intercept-preload/CLAUDE.md` |
| Touch driver-side interception (supervise, collector, build env) | `crates/intercept-supervisor/CLAUDE.md` |
| Touch the shell-completions binary | `crates/bear-completions/CLAUDE.md` |
| Write or modify integration tests | `tests/integration/CLAUDE.md` |
| Edit the test-tooling comparator or normalization | `tests/tools/CLAUDE.md` (behavior in `tests/tools/SPEC.md`) |
| Edit or regenerate the man page | `man/CLAUDE.md` |
| Write or modify a docs site page | `site/CLAUDE.md` |
| Find project documentation or how it is organized | `docs/CLAUDE.md` |
| Add, modify, or review a requirement (contract) | `docs/requirements/CLAUDE.md` |
| Record or look up a design decision, or a rejected option | `docs/rationale/CLAUDE.md` |
| Run an operational procedure (release, ...) | invocable skills in `.claude/skills/` |

Do not skip these reads. They contain constraints that prevent regressions.

## Architecture (data flow)

1. **Interception** - capture compiler invocations via `LD_PRELOAD` (Linux/macOS) or wrapper executable (other platforms)
2. **Semantic analysis** - filter non-compiler commands using interpreter YAML definitions
3. **Configuration** - apply user config for output formatting
4. **Output** - write `compile_commands.json`

## Build pipeline

The workspace builds in three layers before linking the user-facing binaries:

1. `compilers-codegen` runs from `crates/bear/build.rs` to emit interpreter
   tables. See `build-support/compilers-codegen/CLAUDE.md`.
2. `build-support/platform-checks/build.rs` probes the host once for headers
   and symbols. Consumers replay the results via
   `platform_checks::emit_cfg()` / `emit_check_cfg()`. See
   `build-support/platform-checks/CLAUDE.md`.
3. `crates/intercept-preload/build.rs` cc-compiles `src/c/shim.c` and emits
   cdylib link directives. See `crates/intercept-preload/CLAUDE.md`.

`INTERCEPT_LIBDIR` is the one cross-cutting build-time env var
(relative path to the install location of `libexec.so` /
`libexec.dylib`; defaults to `lib`). Validated and forwarded by both
`crates/bear/build.rs` and `tests/integration/build.rs`.

## Host requirements

- `cc` toolchain (gcc or clang).
- `lld` or `mold` linker (ELF platforms, and only when the Rust
  toolchain does not bundle rust-lld; rustup toolchains do). rustc
  injects its own linker version script into the cdylib link next to
  Bear's, and GNU ld refuses to combine the two. The build script
  picks the first available of: bundled rust-lld, system lld, system
  mold (see `docs/rationale/preload-linker-selection.md`). macOS uses
  the system linker.
- `ccache` (optional). When present and on PATH, the integration tests
  exercise a ccache-masquerade-aware test path.

## Decision protocol

For a new feature or architectural change: write a requirement first (see
`docs/requirements/CLAUDE.md`), present a short decision log (approach,
alternatives considered, trade-offs), and wait for approval before
implementing. Develop it test-first (TDD): write the failing test that
cites the requirement, then the code. Before committing a change of this
size, have a review subagent on a different model check it (the `Agent`
tool with a different `model`).

A bug fix that does not change a contract skips straight to test + fix,
citing the governing requirement if one exists. The different-model review
is reserved for feature and architectural changes, not routine fixes.

## Code guidelines

Always-on, language-agnostic standards for every workspace crate:

- Maintain the existing code structure; extend before adding.
- Keep dependencies minimal; a new dependency needs a real justification,
  not convenience.
- No speculative features or over-engineering.

Rust file conventions (edition, SPDX header, error handling, module
structure) live in `.claude/rules/rust.md`, and test conventions in
`.claude/rules/testing.md`. Both are path-scoped to `.rs` files, so they
load when Claude edits Rust. The per-crate `CLAUDE.md` files add only
crate-specific constraints on top of these.

## Commit messages

- Imperative subject under ~70 characters, with a conventional-commit area
  prefix (`docs:`, `fix:`, `feat:`, `chore:`, `test:`, `refactor:`, ...),
  scoped where it sharpens the scan-line (`docs(requirements): ...`).
- Blank line, then a body explaining the *why*; the diff shows the *what*.
- Reference a requirement ID when the commit implements or changes that
  contract, and an issue or PR only when it adds context the body can't.

## Output format

- ASCII only in all files (no em dashes, smart quotes, Unicode bullets)
- Use hyphens, straight quotes, three dots, asterisks/hyphens for bullets

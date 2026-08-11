## Bear crate

This is the application layer. It contains the CLI definitions, configuration,
the modes of operation, and output generation. Compiler recognition and flag
parsing live in `crates/semantic`, which this crate depends on. The
`bear-driver` and `bear-wrapper` binaries live in their own crates
(`crates/bear-driver`, `crates/bear-wrapper`); the shared/agent-side
interception runtime lives in `crates/intercept`, and the driver-side
interception (supervise, TCP collector, build environment) lives in
`crates/intercept-supervisor`.

## Key directories

| Directory | Responsibility |
|---|---|
| `src/modes/` | Modes of operation |
| `src/environment.rs` | Config-to-primitive adapter over `intercept_supervisor::runner` |
| `src/output/` | Output generation (JSON compilation database, statistics) |
| `src/config/` | Configuration loading, validation, types |

## Before modifying

- **CLI arguments** (`src/args.rs`): uses `clap` derive macros. Update man page -- see `man/CLAUDE.md` for instructions.
- **Compiler interpreters**: they are not here. See `crates/semantic/CLAUDE.md`, and `crates/semantic/compilers/CLAUDE.md` before editing YAML.
- **Output format**: check existing integration tests in `tests/integration/` to avoid regressions.
- **Configuration types** (`src/config/types.rs`): changes here affect YAML config parsing. Update validation in `src/config/validation.rs`.

## The config-to-semantic boundary (load-bearing)

`config` is a pure schema layer: it depends on neither `semantic` nor on
codegen output, and it stores a configured `as:` spelling as the string the
user wrote. `semantic` owns the resolution of that spelling onto a compiler
identity. This crate is the single place that sees both sides, and
`Mode::configure` is the single conversion point, which is why an unknown
spelling fails at startup in every mode. Do not resolve a spelling in
`config`, and do not hand a config type to `semantic`. Rationale in
`docs/rationale/workspace-crate-layout.md`.

## Build script

This crate has none. Compiler-table codegen belongs to `crates/semantic`.

The install-layout name vars (`DRIVER_NAME`, `WRAPPER_NAME`, `PRELOAD_NAME`,
`INTERCEPT_LIBDIR`) are emitted by `crates/intercept-supervisor/build.rs`,
which is where `installation.rs` lives and reads them.

## Shell completions

Generated from `clap` definitions at build time:

```sh
target/release/generate-completions target/release/completions
```

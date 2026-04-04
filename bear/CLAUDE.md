## Bear crate

This is the main crate. It contains the CLI driver, semantic analysis, and output generation.

## Key directories

| Directory | Responsibility |
|---|---|
| `src/bin/` | Entry points: `driver.rs` (main), `wrapper.rs`, `generate_completions.rs` |
| `src/modes/` | Modes of operation |
| `src/intercept/` | Command interception orchestration |
| `src/output/` | Output generation (JSON compilation database, statistics) |
| `src/semantic/` | Semantic analysis - compiler detection and flag parsing |
| `src/config/` | Configuration loading, validation, types |
| `interpreters/` | Compiler definition YAML files (see `interpreters/CLAUDE.md`) |

## Before modifying

- **CLI arguments** (`src/args.rs`): uses `clap` derive macros. Update man page -- see `man/CLAUDE.md` for instructions.
- **Compiler interpreters**: read `interpreters/CLAUDE.md` before editing YAML files.
- **Output format**: check existing integration tests in `integration-tests/` to avoid regressions.
- **Configuration types** (`src/config/types.rs`): changes here affect YAML config parsing. Update validation in `src/config/validation.rs`.

## Code generation

`build.rs` reads `interpreters/*.yaml` and generates static Rust arrays via `bear-codegen`.
The generated code is included via `include!()` in the interpreter and recognition modules.

After editing YAML files, run `cargo build` to regenerate, then `cargo test` to validate.

## Shell completions

Generated from `clap` definitions at build time:

```bash
target/release/generate-completions target/release/completions
```

Man pages should also be generated from `clap` via `clap_mangen` (not yet implemented).

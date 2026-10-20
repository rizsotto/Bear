## Bear crate

## Key directories

| Directory | Responsibility |
|---|---|
| `src/modes/` | Modes of operation |
| `src/environment.rs` | Config-to-primitive adapter over `intercept_supervisor::runner` |
| `src/output/` | Output generation (JSON compilation database, statistics) |
| `src/config/` | Configuration loading, validation, types |

## Before modifying

- **CLI arguments** (`src/args.rs`): uses `clap` derive macros. Update the man page; `man/CLAUDE.md` says how. Regenerate `site/src/reference/command-line.md` with `scripts/generate-command-line-reference.py`.
- **Configuration types** (`src/config/types.rs`): changes here affect YAML config parsing. Update validation in `src/config/validation.rs`.

## The config-to-semantic boundary (load-bearing)

`config` is a pure schema layer: it depends on neither `semantic` nor on
codegen output, and it stores a configured `as:` spelling as the string the
user wrote. `semantic` owns the resolution of that spelling onto a compiler
identity. This crate is the single place that sees both sides, and
`Mode::configure` is the single conversion point (startup resolution:
`docs/rationale/compiler-family-definition.md`). Do not resolve a spelling in
`config`, and do not hand a config type to `semantic`. Rationale in
`docs/rationale/workspace-crate-layout.md`.

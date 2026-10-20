## compilers-codegen

## Consumers

Two build scripts call this library, and both read
`crates/semantic/compilers/*.yaml`:

- `crates/semantic/build.rs` via `generate` (the full tables).
- `crates/intercept/build.rs` via `generate_env_keys_only` (compiler
  environment keys only).

## Generated outputs

Snapshot tests in `tests/snapshots/` lock the generated output against
accidental schema drift.

## Adding a compiler

The steps are in `crates/semantic/compilers/CLAUDE.md`. The
`recognize:` checks it lists live in
`yaml_types.rs::RecognizeEntry::validate`.

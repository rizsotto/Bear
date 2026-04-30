## bear-codegen

Build-time code generator for `bear`'s compiler flag tables and
recognition rules.

## How it works

- A regular library, not a `build.rs` itself.
- Invoked from `bear/build.rs` via
  `bear_codegen::generate(flags_dir, out_dir)`.
- Reads `bear/interpreters/*.yaml` (compiler definitions) and writes
  generated Rust source into the consumer's `OUT_DIR`.
- The `bear` crate pulls in the generated code via `include!()` in
  `src/semantic/interpreters/`.

## Generated outputs

The set of generated module names matches the input shape; see
`src/lib.rs::generate` for the current list. YAML schema validation
lives in `yaml_types.rs`. Snapshot tests in `tests/snapshots/` lock
the generated output against accidental schema drift.

## Adding a compiler

Read `bear/interpreters/CLAUDE.md`. After editing YAML, run
`cargo build` to regenerate, then `cargo test` to validate (the
snapshot tests will diff the generated tables).

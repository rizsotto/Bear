## compilers-codegen

Build-time code generator for `bear`'s compiler flag tables and
recognition rules.

## How it works

- A regular library, not a `build.rs` itself.
- Invoked from `crates/bear/build.rs` via
  `compilers_codegen::generate(flags_dir, out_dir)`.
- Reads `crates/bear/compilers/*.yaml` (compiler definitions) and writes
  generated Rust source into the consumer's `OUT_DIR`.
- The `bear` crate pulls in the generated code via `include!()` in
  `src/semantic/interpreters/`.

## Generated outputs

The set of generated module names matches the input shape; see
`src/lib.rs::generate` for the current list. YAML schema validation
lives in `yaml_types.rs`. Snapshot tests in `tests/snapshots/` lock
the generated output against accidental schema drift.

## Adding a compiler

Read `crates/bear/compilers/CLAUDE.md`. After editing YAML, run
`cargo build` to regenerate, then `cargo test` to validate (the
snapshot tests will diff the generated tables). Every `recognize:`
entry needs a `description` and a `references` list -- see
`crates/bear/compilers/CLAUDE.md` for the schema; both are validated
in `yaml_types.rs::RecognizeEntry::validate`.

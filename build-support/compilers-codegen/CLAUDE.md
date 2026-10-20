## compilers-codegen

Build-time code generator for the `semantic` crate's compiler flag tables
and recognition rules.

## How it works

- A regular library, not a `build.rs` itself.
- Invoked from `crates/semantic/build.rs` via
  `compilers_codegen::generate(flags_dir, out_dir)`, and from
  `crates/intercept/build.rs` via `generate_env_keys_only` (compiler
  environment keys only).
- Reads `crates/semantic/compilers/*.yaml` (compiler definitions) and
  writes generated Rust source into the consumer's `OUT_DIR`.
- The `semantic` crate pulls in the generated code via `include!()` in
  `src/interpreters/`.

## Generated outputs

The set of generated module names matches the input shape; see
`src/lib.rs::generate` for the current list. YAML schema validation
lives in `yaml_types.rs`. Snapshot tests in `tests/snapshots/` lock
the generated output against accidental schema drift.

## Adding a compiler

The steps are in `crates/semantic/compilers/CLAUDE.md`. The
`recognize:` checks it lists live in
`yaml_types.rs::RecognizeEntry::validate`.

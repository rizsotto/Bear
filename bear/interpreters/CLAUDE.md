## Compiler interpreter definitions

Read `README.md` in this directory for full schema documentation (pattern syntax,
result values, inheritance, environment variables).

## Rules for modifying YAML files

- Every YAML file maps to one compiler or compiler family
- `build.rs` reads these at build time and generates static Rust arrays
- After any edit: `cargo build && cargo test` to validate

## Adding a new compiler

1. Create `mycompiler.yaml` in this directory
2. Add `type:`, `recognize:`, `flags:` entries (optionally `extends:`, `ignore_when:`, `environment:`)
3. Add a `TableConfig` entry in `bear/build.rs`
4. Add a `CompilerType` variant in `config.rs` and mapping in `compiler_recognition.rs::parse_compiler_type`
5. Register `FlagBasedInterpreter` in `CompilerInterpreter::new_with_config`
6. Run `cargo build && cargo test`

## Adding a new flag to an existing compiler

1. Find the correct YAML file
2. Add entry under `flags:` with `match` pattern and `result`
3. `cargo build` regenerates tables automatically
4. `cargo test` validates sorting and invariants

## Common mistakes

- Forgetting to run `cargo build` after YAML edits (stale generated code)
- Using wrong pattern syntax (see README.md pattern table)
- Adding flags to wrong file when inheritance (`extends:`) would cover it
- Not considering cross-platform implications (`slash_prefix` for MSVC-style compilers)

## Regression protection

Compiler interpreter changes must be covered by integration tests.
See `integration-tests/CLAUDE.md` for how to write them.

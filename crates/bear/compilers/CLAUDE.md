## Compiler interpreter definitions

Read `README.md` in this directory for full schema documentation (pattern syntax,
result values, inheritance, environment variables).

## Rules for modifying YAML files

- Every YAML file maps to one compiler or compiler family
- `compilers-codegen` reads these at build time (via `crates/bear/build.rs`) and generates static Rust arrays
- After any edit: `cargo build && cargo test` to validate

## Adding a new compiler

1. Create `mycompiler.yaml` in this directory
2. Add `type:`, `recognize:`, `flags:` entries (optionally `extends:`, `ignore_when:`, `environment:`)
3. Add a `TableConfig` entry in `build-support/compilers-codegen/src/tables.rs`
4. Add a `CompilerType` variant in `crates/bear/src/config/types.rs` and a mapping in `crates/bear/src/semantic/interpreters/compilers/compiler_recognition.rs::parse_compiler_type`
5. Add a constructor in `flag_based.rs` and register it in `CompilerInterpreter::new_with_config` (`crates/bear/src/semantic/interpreters/compilers/mod.rs`)
6. Run `cargo build && cargo test`

## `recognize:` entries must be documented and cited

Every `recognize:` entry is mandatory `description` (a short human label,
e.g. `"GCC"`) and a mandatory `references` (a non-empty list of
http(s) documentation URLs). Both are validated at codegen time -- the
build fails if either is missing, blank, or `references` holds a
non-http(s) entry. `description` is emitted into the generated
`RECOGNITION_PATTERNS` table and shown by `bear semantic --print-compilers`;
`references` is validate-only and is never emitted into generated code.

## Adding a new flag to an existing compiler

1. Find the correct YAML file
2. Add entry under `flags:` with `match` pattern and `result`
3. `cargo build` regenerates tables automatically
4. `cargo test` validates sorting and invariants

## Properties set in the factory, not the YAML

A few per-interpreter properties are consumed at the converter (post-parse),
not at parse time, so they are hard-coded in the factory functions in
`flag_based.rs` rather than in these YAML files:

- `source_mode` (`SourceMode`, default `PerSourceStripped`): controls how an
  invocation's sources map to database entries. `PerSourceStripped` for most
  families (one entry per source, siblings stripped); `Combined` for a
  single-translation-unit compiler like `valac` (one combined entry per
  invocation, all sources retained); `PerSourceFull` for a whole-module
  compiler like `swiftc` (one entry per source, but every entry keeps every
  source). See `README.md` for details.

## Common mistakes

- Forgetting to run `cargo build` after YAML edits (stale generated code)
- Using wrong pattern syntax (see README.md pattern table)
- Adding flags to wrong file when inheritance (`extends:`) would cover it
- Not considering cross-platform implications (`slash_prefix` for MSVC-style compilers)

## Regression protection

Compiler interpreter changes must be covered by integration tests.
See `tests/integration/CLAUDE.md` for how to write them.

## Compiler interpreter definitions

Read `README.md` in this directory for full schema documentation (pattern syntax,
result values, inheritance, environment variables).

## Two schemas live side by side here

`type:` is a closed kind enum. Compiler files (`type: compiler`) declare a
family identity in a nested `compiler:` block (`id:`, optional `extends:`).
Wrapper files (`type: wrapper` -- the four compiler launchers: ccache,
distcc, sccache, icecc) carry no identity block. Codegen discovers both
kinds by scanning this directory and peeking `type:` -- there is no
hand-maintained file list. See `README.md` for the full schema of both.

## Rules for modifying YAML files

- Every compiler-kind YAML file maps to one compiler or compiler family;
  every wrapper-kind file maps to one compiler launcher
- `compilers-codegen` reads these at build time (via `crates/bear/build.rs`) and generates static Rust arrays
- After any edit: `cargo build && cargo test` to validate

## Adding a new compiler

Adding a family is a YAML file plus accepting snapshots -- no Rust edit.
The directory scan picks up the file, derives every generated name from
its stem, and emits the recognition row, the `KNOWN_IDS` entry (so
`as: <id>` is accepted), and the interpreter registration on its own.

1. Create `mycompiler.yaml` in this directory
2. Add `type: compiler`, a `compiler:` block (`id:`, optionally `extends:`), `recognize:`, `flags:` entries (optionally `ignore_when:`, `environment:`, `slash_prefix:`, `source_mode:`, `response_file_syntax:`). `id:` must be unique across this directory (checked at codegen) and is the only accepted config `as:` spelling for this family -- no aliases.
3. Run `cargo build && cargo test`, then accept the snapshot diff (`cargo insta accept`)
4. Write the family's page under `site/src/guides/recipes/` in the same release, and list it in `site/src/SUMMARY.md` and `site/src/guides/recipes/index.md`. Recognizing a toolchain is only half the feature: nobody searching for that toolchain finds Bear until a page says its name. See `site/CLAUDE.md` for the authoring rules, and use the family's `id:` when the page shows an `as:` value.

## Adding a new wrapper

1. Create `mywrapper.yaml` with `type: wrapper`, a `recognize:` entry, and optionally `options:` (exact-token launcher flags only)
2. No Rust change needed: codegen discovers it by kind, the unwrap logic is generic over generated data, and the basename is emitted into `WRAPPER_AS_NAMES` so `as: mywrapper` is accepted -- see `README.md`
3. Run `cargo build && cargo test`, then accept the snapshot diff

## Removing a compiler

Delete the YAML file, then delete its now-orphaned per-family flag snapshot
(`build-support/compilers-codegen/tests/snapshots/snapshots__snapshot_flags_<stem>.snap`)
by hand: the looped snapshot test names snapshots per discovered stem, so a
removed family's stored snapshot is simply no longer referenced, not
reported as stale.

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

## Per-family selectors: source_mode and response_file_syntax

Two selectors are consumed outside the flag classifier (`source_mode` at
the converter, `response_file_syntax` at response-file tokenization).
Their semantics are code; the per-family choice is data -- optional
top-level YAML keys, not inherited through `extends`. Defaults:
`source_mode: per-source-stripped` (vala is `combined`, swift is
`per-source-full`) and `response_file_syntax: gnu` (msvc and clang_cl are
`msvc`). See `README.md` for the full description of each value.

## Common mistakes

- Forgetting to run `cargo build` after YAML edits (stale generated code)
- Using wrong pattern syntax (see README.md pattern table)
- Adding flags to wrong file when inheritance (`extends:`) would cover it
- Not considering cross-platform implications (`slash_prefix` for MSVC-style compilers)

## Regression protection

Compiler interpreter changes must be covered by integration tests.
See `tests/integration/CLAUDE.md` for how to write them.

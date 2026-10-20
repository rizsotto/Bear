# A compiler family is defined by its YAML file

## Context

Bear defines each compiler family and launcher in a YAML file, and
codegen turns the files into static tables at build time. The YAML
always held the flag tables. The rest of a family was spread across
hand-written Rust: a list of the files, a `CompilerType` enum with one
variant per family (the config `as:` surface), a string-to-enum map that
panicked on an unknown value, a constructor and a registration call per
family, and the `source_mode` and response-file syntax set inside those
constructors. Adding a family took six edits across two crates. Each
missed edit failed in its own way: a silent drop, a compile error, or a
runtime panic.

The config `as:` value had its own problem. The enum took many spellings
per family through hand-written serde aliases, plus whatever
`rename_all = "lowercase"` made of a variant name (`clangcl` for
`ClangCl`). Nobody designed that surface. It grew as families were
added, and there was no record of who used which spelling.

Options we did not take:

- **Keep the per-family enum.** It was a hand-synced copy of the ids
  codegen already emits. Its one reason to exist was the alias surface,
  which this decision drops. Dispatch never relied on Rust
  exhaustiveness: it is a runtime registry with a completeness test.
- **Generate the enum from the data.** Codegen would learn an
  id-to-variant mapping, and the enum would keep its per-variant
  ceremony while still copying the data. Dropping the type is simpler.
- **A YAML `aliases:` list per family.** It generates a bigger copy of
  a surface nobody designed. No user had asked for any alias.
- **Hyphenated ids for display** (`clang-cl` for `clang_cl` in
  `--print-compilers`). Once there are no aliases, any other display
  prints a value that differs from the one users must type.
- **A recognition `priority` field.** The only real order rule is
  "a specialization before its base", and that is the `extends` link a
  vendor variant declares anyway for flag inheritance. A separate number
  would be one more thing to forget.

## Decision

A compiler family is defined only by its YAML file. Codegen finds the
files by scanning the directory, derives every generated name from the
file stem, and emits the family registry and the id list. `CompilerType`
has two variants: a compiler carrying an id checked against the
generated list, or a wrapper. A family that `extends` another is
recognized before it. `source_mode` and `response_file_syntax` are YAML
data; what each value means stays in code.

Each family accepts one `as:` spelling: its `compiler.id`, verbatim, in
the YAML, the generated data, `--print-compilers`, and config parsing. A
launcher has no `compiler.id`, so its basename is the accepted spelling.
The basenames are generated from the launcher YAML too, and resolve to
the wrapper kind like `wrapper` does. The config
schema keeps the string the user wrote. Bear resolves it once at
startup, in every mode that reads configuration, against the generated
ids. So a typo fails before any build runs or input is read, and the
error lists every accepted value.

## Consequences

- Adding a family or a launcher is a YAML file plus accepted snapshots.
  No Rust edit, and no parallel list left to drift.
- The `as` column of `--print-compilers` is exactly what a user types
  in `as:`. There is no display transform to keep in sync.
- Configs that used a dropped alias (`llvm`, `gnu`, `clangcl`,
  `intel-cc`, and others) now fail at startup.
- What stays code, on purpose: the meaning of each `source_mode` and
  response-file syntax value, the `cc`/`c++` version probe, the
  ambiguous-name set, and the source-extension table. These are
  behavior, not per-family data.
- Revisit only with real user demand for a second spelling. Both
  aliases and display spellings were drafted in full and dropped.

## References

- [recognition-compiler-names](../requirements/recognition-compiler-names.md),
  the contract over whatever the data defines
- [recognition-compiler-launchers](../requirements/recognition-compiler-launchers.md),
  the launcher contract over the launcher data
- [workspace-crate-layout](workspace-crate-layout.md), why the
  conversion from config string to compiler identity lives in the
  application crate

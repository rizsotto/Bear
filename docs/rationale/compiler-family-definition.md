# A compiler family is defined by its YAML file

## Context

Bear recognizes roughly nineteen compiler families and four launchers,
defined in per-family YAML and compiled to static tables at build time.
The YAML always held the flag tables, but the rest of a family was spread
across hand-maintained Rust: an array listing each file and its generated
static names, a nineteen-variant `CompilerType` enum (the config `as:`
surface), a string-to-enum map that panicked on an unknown value, a
per-family interpreter constructor, a registration call, and per-family
`source_mode`/response-file-syntax baked into those constructors. Adding a
family meant six edits across two crates, and forgetting one failed a
different way each time: a silent drop, a compile error, or a runtime
panic.

The recognized set and each family's flag semantics were already meant to
live in the data (see `recognition-compiler-names`); the *enumeration of
families* was not.

## Decision

A compiler family is defined solely by its YAML file. Codegen discovers
the files by scanning the directory and peeking each file's kind; every
generated name derives from the file stem; and generated registries (the
family registry and the id list) replace the hand-maintained enum, map,
constructors, and registration.

The runtime type mirrors the YAML schema, not the family list:
`CompilerType` is a two-variant kind (compiler-carrying-an-id, or
wrapper), where the id is a static string validated against the generated
id list. There is no per-family variant.

Recognition order -- which family a name resolves to when patterns overlap
-- derives from the `extends` graph: a family that extends another is a
specialization of it, so it is recognized first. There is no hand-set
priority number.

Two per-family selectors that used to be factory-set, `source_mode` and
`response_file_syntax`, become YAML data (their semantics stay in code).
This reverses the earlier "set in the factory, not the YAML" guidance.

## Consequences

- Adding a family is a YAML file plus accepting snapshots -- no Rust edit,
  nothing to forget. Adding a launcher is likewise YAML-only.
- The six sync points and their three failure modes are removed, not
  converted: there is no parallel list left to drift from the data.
- The public config surface (`as:`) accepts exactly each family's id,
  verbatim (see `compiler-as-no-aliases`); the spelling is resolved at
  startup against the generated data, and the error names every
  accepted value.
- What stays code, by design: the meaning of each `source_mode` and
  response-file-syntax value, the `cc`/`c++` version probe and its
  gcc/clang verdicts, the ambiguous-name set, and the source-extension
  table. These are behavior, reviewed as code, not per-family data.

### Rejected alternatives

- **Keep the nineteen-variant `CompilerType`.** It was a hand-synced
  mirror of ids codegen already emits; its original justification (a
  curated `as:` alias surface) was dropped separately
  (`compiler-as-no-aliases`), and Rust-level exhaustiveness was never
  relied on -- dispatch is a runtime registry with a completeness test.
  Someone will propose re-adding a per-family variant; this entry is why
  it is gone.
- **Generate the enum from the data.** Purity without leverage: codegen
  would learn an id-to-variant mapping and the enum would keep its
  per-variant ceremony while still mirroring the data. Dissolving the
  type into the data is the simpler inverse.
- **A recognition `priority` field.** Drafted, then dropped: the only
  real ordering constraint (specialization before base) is exactly the
  `extends` relationship, so a separate number is redundant, and a new
  vendor variant of clang/gcc would have to remember to set it.
  `extends` -- which such a family declares anyway for flag inheritance
  -- gives correct order for free.

## References

- Requirement: `recognition-compiler-names` (the behavioral contract over
  whatever the data defines).
- `compiler-as-no-aliases` (the one-spelling-per-family `as:` decision
  this builds on).

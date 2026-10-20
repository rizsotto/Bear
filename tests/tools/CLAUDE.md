## tests/tools

Shared test tooling. The library exposes the compilation-database comparison
and normalization logic; the binary `cdb-compare` wraps it.

The behavioral contract for `cdb-compare` (subcommands, normalization flags,
invariants checks, JSON shapes, sampler) lives in `SPEC.md`. Read it before
changing what the tool does, and keep it in sync when you change behavior.

## Single implementation

`tests/integration` depends on this library so there is one comparison
implementation, not two. `CompilationEntryMatcher` lives here and is
re-exported by the integration fixtures; the dogfooding harness layers on the
same library. Do not fork a second comparator - extend this one.

## Constraints

- Prefer crates already in the workspace over new ones.
- Tests are host-independent: they operate on in-memory or small fixture CDB
  JSON, never a live build or container.

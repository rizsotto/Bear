## semantic

The per-module `//!` docs hold the details; this file lists only the
cross-cutting constraints that are easy to violate.

## Dependency boundary (load-bearing)

May depend on `intercept`. Must NOT depend on `bear`, and must not grow a
dependency on the application layer in any form: no configuration types,
no `clap`, no command-line arguments, no serde. A caller's configuration
reaches this crate as plain values: a path, an optional `as:` spelling, a
boolean. Rationale in `docs/rationale/workspace-crate-layout.md`.

Resolution of a configured `as:` spelling onto a `CompilerType` belongs
here, next to the generated id data it validates against, not in the
caller's schema layer.

## The public API is deliberate

`src/lib.rs` states exactly what dependents may touch; everything else is
crate-private, including `CompilerType`, `CompilerId`, `CompilerInterpreter`,
the interpreter combinators, and `IgnoreByPath`. Widening that surface is
a design change, not a mechanical fix: say in the commit body why the
dependent cannot be served by what is already exported. Composition of
the interpreter chain is a semantic concern and stays behind
`interpreters::create`.

## The `testing` feature

The `testing` module is gated on `any(test, feature = "testing")`, not on
`cfg(test)`. Enable the feature from a dependent's `[dev-dependencies]`
only. Do not copy a factory into the dependent. Rationale in
`docs/rationale/workspace-crate-layout.md`.

## Compiler definitions are data

Recognition patterns, flag tables, the family registry, and the accepted
`as:` spellings are all generated from `compilers/*.yaml` by
`build.rs`. Read `compilers/CLAUDE.md` before editing YAML (it owns the
steps to add a compiler or launcher), and
`build-support/compilers-codegen/CLAUDE.md` before changing what is
generated. Hand-writing a compiler id as a Rust literal is guarded by
`compiler_id_literals_are_known`; keep it that way.

## semantic

The analysis core: recognizes which executions are compiler invocations,
and parses their command lines into classified arguments. It owns the
`compilers/*.yaml` definitions and the codegen build script that turns
them into static tables.

The per-module `//!` docs hold the details; this file lists only the
cross-cutting constraints that are easy to violate.

## Dependency boundary (load-bearing)

May depend on `intercept`. Must NOT depend on `bear`, and must not grow a
dependency on the application layer in any form: no configuration types,
no `clap`, no command-line arguments, no serde. The whole reason this is
a crate rather than a module is that the compiler now rejects that edge
instead of a reviewer having to catch it. A caller's configuration
reaches this crate as plain values: a path, an optional `as:` spelling, a
boolean. Rationale in `docs/rationale/workspace-crate-layout.md`.

Resolution of a configured `as:` spelling onto a `CompilerType` belongs
here, next to the generated id data it validates against, not in the
caller's schema layer.

## The public API is deliberate

`src/lib.rs` states exactly what dependents may touch; everything else is
`pub(crate)`, including `CompilerType`, `CompilerId`, `CompilerInterpreter`,
the interpreter combinators, and `IgnoreByPath`. Widening that surface is
a design change, not a mechanical fix: say in the commit body why the
dependent cannot be served by what is already exported. Composition of
the interpreter chain is a semantic concern and stays behind
`interpreters::create`.

## The `testing` feature

`src/testing.rs` is gated on `any(test, feature = "testing")`, not on
`cfg(test)`. A dependency's `cfg(test)` code is not compiled when the
dependent's tests build, and `bear`'s output tests use these command
factories and comparison helpers. The feature is off by default, so
release builds carry no test factories. Enable it from a dependent's
`[dev-dependencies]` only; duplicating a factory in the dependent lets
the two drift.

## Compiler definitions are data

Recognition patterns, flag tables, the family registry, and the accepted
`as:` spellings are all generated from `compilers/*.yaml` by
`build.rs`. Adding a compiler family or a launcher is a YAML file plus
accepted snapshots, with no Rust edit. Read `compilers/CLAUDE.md` before
editing YAML, and `build-support/compilers-codegen/CLAUDE.md` before
changing what is generated. Hand-writing a compiler id as a Rust literal
is guarded by `compiler_id_literals_are_known`; keep it that way.

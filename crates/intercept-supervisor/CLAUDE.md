## intercept-supervisor

Driver-side half of the interception runtime: supervises the build,
collects executions over TCP, and sets up the build environment
(wrapper directory or preload library).

The per-module `//!` docs hold the details; this file lists only the
cross-cutting constraints that are easy to violate.

## Dependency boundary (load-bearing)

May depend on `intercept`. Must NOT depend on `bear`, `config`, or
`clap`. The preload cdylib depends on `intercept` only; keeping
supervisor-only deps (`signal-hook`, `which`) out of `intercept`'s
graph is what stops them from leaking into that cdylib. Adding such an
edge here is fine; pushing one down into `intercept` is not.

## Sibling location is relative-path-only

`installation.rs` finds `bear-wrapper` and `libexec.so` via relative
paths from the current executable. Never hard-code or compute absolute
install paths; `INTERCEPT_LIBDIR` is the one build-time knob.

`build.rs` reads `INTERCEPT_LIBDIR` (default `lib`), trims it, rejects
an empty or absolute value, accepts the literal `$LIB`, and passes it to
`installation.rs` through `env!()`. The preload library sits at
`../$INTERCEPT_LIBDIR/` from the driver's `bin/` directory.
`tests/integration/build.rs` reads the same variable with the same
default and forwards it to the tests. Its own check rejects an empty or
absolute value, but it does not trim first and has no `$LIB` case
(`$LIB` passes as a relative path). Keep the two in step. The
user-facing side (`$LIB`, matching the install step) is in `INSTALL.md`.

## Process-group nesting

Supervision chains nest in wrapper mode, so only the outermost
supervisor creates a process group (`GroupPolicy::Leader` in
`supervise.rs`); nested ones `Inherit`. Governed by requirement
`cli-signal-forwarding`; rationale in
`docs/rationale/process-tree-teardown.md`.

## Wrapper directory

The wrapper dir is the deterministic `.bear/` in the build cwd (not a
temp dir) and is wiped at the start of each run (`wrapper.rs`).
Rationale in `docs/rationale/wrapper-mode-design-decisions.md`.

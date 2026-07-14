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

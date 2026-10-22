## intercept-supervisor

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
`installation.rs` through `env!()`. `tests/integration/build.rs` reads the
same variable and default and rejects an empty or absolute value. It does
not trim before the absolute check and has no `$LIB` case. Keep the two
in step. The user-facing side is in `INSTALL.md`.

## Process-group nesting

Supervision chains nest in wrapper mode, so only the outermost
supervisor creates a process group (`GroupPolicy::Leader` in
`supervise.rs`); nested ones `Inherit`. Governed by requirement
`cli-signal-forwarding`; rationale in
`docs/rationale/supervise-a-compiler-call.md`.

## Wrapper directory

The wrapper dir is the deterministic `.bear/` in the build cwd (not a
temp dir) and is wiped at the start of each run (`wrapper.rs`).
Rationale in `docs/rationale/wrapper-mode-design-decisions.md`.

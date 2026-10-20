# The completions generator is its own crate

## Context

Bear ships shell-completion scripts. A small binary writes them from the
`clap` definitions of the command line, using `clap_complete`. Nothing
else in Bear needs `clap_complete`.

The obvious place for that binary is the application crate, next to the
`clap` definitions it reads. But a binary inside that crate puts
`clap_complete` in the crate's dependency graph for every build. That
includes the driver and wrapper binaries users run, which never need it,
and it adds compile time to each of them.

## Decision

The generator lives in its own crate. It depends on the application
crate for the `clap` builder, and nothing depends on it.

## Consequences

- `clap_complete` is built only when the generator is built.
- The dependency edge points one way, from the generator to the
  application crate.
- A plain `cargo build` of the main install does not make completions.
  Making them is a separate step.

## References

- [CLAUDE.md](../../CLAUDE.md), the crate roles
- [bear-completions/CLAUDE.md](../../crates/bear-completions/CLAUDE.md), how the generator is run

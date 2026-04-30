## bear-completions

Standalone crate housing the `generate-completions` binary.

## Why a separate crate

`generate-completions` is the only consumer of `clap_complete`
(~0.5s of compile time). Keeping it inside `bear` would put
`clap_complete` in `bear`'s dependency graph for every build,
including the user-facing driver and wrapper binaries that don't
need it.

## How it's invoked

Not by `cargo build` for the main install. The distributor runs the
binary explicitly (`INSTALL.md` documents the command); it writes
shell-completion files into a directory which `scripts/install.sh`
then picks up if present. The install script does not run the
generator itself.

## Dependency on bear

Depends on `bear` only for `bear::args::cli()` (the `clap` builder).
This is a one-way edge from `bear-completions` to `bear`; nothing in
`bear` references `bear-completions`.

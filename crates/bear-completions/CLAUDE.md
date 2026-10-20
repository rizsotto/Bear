## bear-completions

The reason this is a separate crate is in
`docs/rationale/completions-own-crate.md`.

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

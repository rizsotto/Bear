<!-- Diataxis type: how-to -->

# Install Bear

Install Bear with your platform's package manager:

```sh
# Debian / Ubuntu
sudo apt install bear

# Fedora
sudo dnf install bear

# Arch Linux
sudo pacman -S bear

# macOS (Homebrew)
brew install bear

# FreeBSD
pkg install bear
```

Then confirm it is on your `PATH`:

```sh
bear --version
```

Compare that version against the [latest release][latest-release].
Distribution packages often lag several releases behind, so a problem
you hit may already be fixed upstream; build the current release from
source before reporting a bug.

For the full list of distributions that carry a Bear package, see the
[Repology page][repology]. Repology tracks the version each distribution
ships; if yours lags behind, or does not package Bear at all, build it
from source instead.

## Build from source

Follow [`INSTALL.md`][install] in the repository: it covers
prerequisites (the Rust toolchain and a C compiler), building, and
installing with `cargo build --release` and `./scripts/install.sh`,
including custom install prefixes and packaging notes. This page does
not duplicate those steps.

## Platform-specific notes

A distribution package or a source build both give you a working
`bear`, but a few platforms have their own constraints on how Bear
intercepts a build:

- [Bear on Linux, WSL2, and Docker](platforms/linux.md)
- [Bear on macOS](platforms/macos.md)
- [Bear on Windows](platforms/windows.md)
- [Bear on BSD](platforms/bsd.md)

## Next steps

Once `bear --version` works, follow
[Getting started with Bear](getting-started.md) for the first real run,
or jump straight to a [recipe](recipes/index.md) for your build system.

  [repology]: https://repology.org/project/bear-clang/versions
  [install]: https://github.com/rizsotto/Bear/blob/master/INSTALL.md
  [latest-release]: https://github.com/rizsotto/Bear/releases/latest

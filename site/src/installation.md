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
source before reporting a bug. `bear --help`, or [Command-line
options](reference/command-line.md), lists every flag once you have it
installed.

For the full list of distributions that carry a Bear package, see the
[Repology page][repology]. Repology tracks the version each distribution
ships; if yours lags behind, or does not package Bear at all, build it
from source instead. Bear is not published on crates.io, so `cargo
install bear` will not find it; `cargo build --release` plus the
install script below is the source-build path.

## Build from source

Follow [`INSTALL.md`][install] in the repository for prerequisites (the
Rust toolchain and a C compiler) and the full steps. The short version:

```sh
cargo build --release
./scripts/install.sh
```

`scripts/install.sh` installs to `/usr/local` when run as root (for
example under `sudo`), or `$HOME/.local` otherwise - a plain, unelevated
run already gives you a working, no-`sudo` install. Override either
default explicitly with `PREFIX`:

```sh
sudo PREFIX=/usr ./scripts/install.sh
```

## `DESTDIR` for packaging, separately from `PREFIX`

`scripts/install.sh` accepts two different path variables, and they are
not interchangeable. `PREFIX` is the final, on-system path; the `bear`
entry script it installs embeds that path literally, so `PREFIX` must
match where the package actually runs, not where it is being built.
`DESTDIR`, when set, is a staging root prepended to every install path
on top of `PREFIX`, for building a package without touching the real
system; it must be an absolute path. Building a distribution package
sets both together:

```sh
DESTDIR=/tmp/pkgroot PREFIX=/usr ./scripts/install.sh
```

## `INTERCEPT_LIBDIR` must match between build and install

On systems that use `lib64` or another non-default library directory
name, set `INTERCEPT_LIBDIR` for both the build and the install step,
so `bear-driver` finds the preload library where it actually ends up:

```sh
INTERCEPT_LIBDIR=lib64 cargo build --release
INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
```

Let the two commands disagree and Bear still installs without
complaint, but preload interception silently fails at runtime: the
dynamic linker cannot find the library at the path `bear-driver`
expects, so it skips loading it and lets the build run anyway. Nothing
about that failure is loud - the build succeeds, Bear exits `0`, and
the only symptom is a `compile_commands.json` that came out empty or
suspiciously short, because a missing preload library is not one of the
failures Bear itself reports (see [Exit
status](reference/exit-status.md#notes)). [Bear on
Linux](platforms/linux.md#installing-the-preload-library-to-a-non-standard-lib-directory)
has the matching commands for the common `lib64` case.

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
[Getting started with Bear](tutorials/getting-started.md) for the first real run,
or jump straight to a [recipe](guides/recipes/index.md) for your build system.

  [repology]: https://repology.org/project/bear-clang/versions
  [install]: https://github.com/rizsotto/Bear/blob/master/INSTALL.md
  [latest-release]: https://github.com/rizsotto/Bear/releases/latest

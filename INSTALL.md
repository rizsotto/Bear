# Install from distribution package

Bear has been around for a while, and packages are available in many
distributions. For an easy installation, consult your distribution's package
manager. These packages are well-tested and should be the first choice for
installation.

Common package manager commands:

   ```bash
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

For a full list of available packages, see the
[Repology page](https://repology.org/project/bear-clang/versions).

# Install from source

If the latest version is not available in your distribution, install Bear from
source. Follow the steps below.

## Prerequisites

Bear is now implemented in Rust, so the Rust toolchain is required.

**Rust toolchain** (1.85 or later): Bear uses the Rust 2024 edition, which
requires Rust 1.85+. Install Rust using [rustup](https://rustup.rs/):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

Ensure that `cargo` and `rustc` are available in your `PATH`:

   ```bash
   rustc --version   # must be >= 1.85
   cargo --version
   ```

**C compiler**: A C compiler is required to build the preload library
(`intercept-preload`). The `cc` crate will typically find one automatically;
ensure `gcc` or `clang` is installed.

## Simple installation

1. Clone the repository:
   ```bash
   git clone https://github.com/rizsotto/Bear.git
   cd Bear
   ```

2. Build:
   ```bash
   cargo build --release
   ```

3. Install:
   ```bash
   ./scripts/install.sh
   ```

4. Verify the installation:
   ```bash
   bear --version
   bear -- true   # quick smoke test — should produce an empty compile_commands.json
   ```

## Uninstall

Once installed, the easiest way to remove all files is to run the original
install script with the uninstall option:

   ```bash
   ./scripts/install.sh --uninstall
   ```

If the source tree is no longer available, run the `uninstall.sh` script:

   ```bash
   sh $HOME/.local/share/bear/uninstall.sh
   ```

The path above assumes Bear was installed under `$HOME/.local`. Depending on
customization, the uninstall script may be located elsewhere.

## Custom installation

By default, Bear is installed to `/usr/local` (when run as root) or
`$HOME/.local` (otherwise). You can override the installation prefix with
`PREFIX`:

   ```bash
   sudo PREFIX=/usr ./scripts/install.sh
   ```

`PREFIX` is the final install location (e.g., `/usr`, `/usr/local`,
`$HOME/.local`). Binaries go into `$PREFIX/bin/`, libraries into
`$PREFIX/share/bear/`, and so on.

The preload library directory name defaults to `lib`. On systems where a
different directory is needed, set `INTERCEPT_LIBDIR` at both build and
install time:

   ```bash
   # Build with the correct library directory compiled in
   INTERCEPT_LIBDIR=lib64 cargo build --release

   # Install with the same value so the file is placed where bear-driver expects it
   INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
   ```

On glibc-based Linux, the special value `$LIB` can be used — the dynamic
linker expands it at runtime (see `man ld.so`). On other platforms (macOS,
musl, FreeBSD), use a concrete directory name.


# How to package

If you are a package maintainer for a distribution:

- Build and install with explicit values for `PREFIX` and `INTERCEPT_LIBDIR`:
  ```bash
  INTERCEPT_LIBDIR=lib64 cargo build --release
  INTERCEPT_LIBDIR=lib64 PREFIX=$pkgdir/usr ./scripts/install.sh
  ```

- The preload library (`libexec.so`) is only built on Unix. Windows builds
  only produce `bear-driver` and `bear-wrapper`. Consult
  `intercept-preload/build.rs` for details.

- `bear-driver` locates its siblings using relative paths:
  `./bear-wrapper` and `../$INTERCEPT_LIBDIR/libexec.so`. The `bear` entry
  script in `$PREFIX/bin/` is the only artifact that uses an absolute path.

- The expected installation layout:
  ```
  $PREFIX/
  ├── bin/
  │   └── bear                        (shell script)
  └── share/
      ├── doc/
      │   └── bear/
      │       ├── README.md
      │       └── COPYING
      ├── man/
      │   └── man1/
      │       └── bear.1
      └── bear/
          ├── bin/
          │   ├── bear-driver
          │   └── bear-wrapper
          └── $INTERCEPT_LIBDIR/
              └── libexec.so
  ```

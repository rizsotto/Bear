# How to install

Bear has been around for a while, and packages are available in many
distributions. For an easy installation, check your machine's package manager
for available packages. These packages are well-tested and should be your first
choice for installation.

# How to build

Bear is now implemented in Rust and can be built and installed using the Rust
toolchain.

## Prerequisites

**Rust toolchain**: Install Rust using [rustup](https://rustup.rs/).
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

Ensure `cargo` and `rustc` are available in your PATH:

   ```bash
   rustc --version
   cargo --version
   ```

## Build and Install

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

By default, Bear is installed to `/usr/local` (when run as root) or
`$HOME/.local` (otherwise). You can override this with `DESTDIR`:

   ```bash
   sudo DESTDIR=/usr/local ./scripts/install.sh
   ```

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

## Uninstall

   ```bash
   ./scripts/install.sh --uninstall
   ```

The install script writes a manifest at `$DESTDIR/share/bear/install-manifest.txt`.
The `--uninstall` flag reads this manifest and removes only the files that were
previously installed.

# How to package

If you are a package maintainer for a distribution:

- Build and install with explicit values for `DESTDIR` and `INTERCEPT_LIBDIR`:
  ```bash
  INTERCEPT_LIBDIR=lib64 cargo build --release
  INTERCEPT_LIBDIR=lib64 DESTDIR=$pkgdir/usr ./scripts/install.sh
  ```

- The preload library (`libexec.so`) is only built on Unix. Windows builds
  only produce `bear-driver` and `bear-wrapper`. Consult
  `intercept-preload/build.rs` for details.

- `bear-driver` locates its siblings using relative paths:
  `./bear-wrapper` and `../$INTERCEPT_LIBDIR/libexec.so`. The `bear` entry
  script in `$DESTDIR/bin/` is the only artifact that uses an absolute path.

- The expected installation layout:
  ```
  $DESTDIR/
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

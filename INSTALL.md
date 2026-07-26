# Install from distribution package

Bear has been around for a while, and packages are available in many
distributions. For an easy installation, consult your distribution's package
manager. These packages are well-tested and should be the first choice for
installation.

Common package manager commands:

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

For a full list of available packages, see the
[Repology page](https://repology.org/project/bear-clang/versions).

# Install from source

If the latest version is not available in your distribution, install Bear from
source. Follow the steps below.

## Prerequisites

Bear is now implemented in Rust, so the Rust toolchain is required.

**Rust toolchain** (1.85 or later): Bear uses the Rust 2024 edition, which
requires Rust 1.85+. Install Rust using [rustup](https://rustup.rs/):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

Ensure that `cargo` and `rustc` are available in your `PATH`:

   ```sh
   rustc --version   # must be >= 1.85
   cargo --version
   ```

**C compiler**: A C compiler is required to build the preload library
(`intercept-preload`). The `cc` crate will typically find one automatically;
ensure `gcc` or `clang` is installed.
On Linux/ELF platforms the preload library links with `lld` or `mold`;
GNU `ld` cannot combine the Rust compiler's linker version script with
Bear's. Toolchains installed via rustup bundle `rust-lld`, which the
build picks up automatically; otherwise install `lld` or `mold`.

## Simple installation

1. Clone the repository:
   ```sh
   git clone https://github.com/rizsotto/Bear.git
   cd Bear
   ```

2. Build:
   ```sh
   cargo build --release
   ```

3. (Optional) Generate shell completions:
   ```sh
   target/release/generate-completions target/release/completions
   ```

4. Install:
   ```sh
   ./scripts/install.sh
   ```

   If the completions directory exists under the build artifacts, the install
   script will automatically install completions for bash, zsh, fish, and
   elvish to standard locations.

5. Verify the installation:
   ```sh
   bear --version
   bear -- true   # quick smoke test - should produce an empty compile_commands.json
   ```

## Uninstall

The install script does not generate an uninstall script. To remove Bear,
delete the installed files manually. The installation layout is listed at the
end of this document.

## Custom installation

By default, Bear is installed to `/usr/local` (when run as root) or
`$HOME/.local` (otherwise). You can override the installation prefix with
`PREFIX`:

   ```sh
   sudo PREFIX=/usr ./scripts/install.sh
   ```

`PREFIX` is the final install location (e.g., `/usr`, `/usr/local`,
`$HOME/.local`). Binaries go into `$PREFIX/bin/`, libraries into
`$PREFIX/libexec/bear/`, and so on.

The preload library directory name defaults to `lib`. On systems where a
different directory is needed, set `INTERCEPT_LIBDIR` at both build and
install time:

   ```sh
   # Build with the correct library directory compiled in
   INTERCEPT_LIBDIR=lib64 cargo build --release

   # Install with the same value so the file is placed where bear-driver expects it
   INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
   ```

On glibc-based Linux, the special value `$LIB` can be used - the dynamic
linker expands it at runtime (see `man ld.so`). On other platforms (macOS,
musl, FreeBSD), use a concrete directory name.

### Shell completions with a user-local install

When Bear is installed to a system prefix like `/usr` or `/usr/local`, shells
typically find completions automatically. For a user-local install
(`$HOME/.local`), you need to tell your shell where to look:

**Bash** - add to `~/.bashrc`:
   ```bash
   source "$HOME/.local/share/bash-completion/completions/bear"
   ```

**Zsh** - add to `~/.zshrc` (before `compinit`):
   ```zsh
   fpath=("$HOME/.local/share/zsh/site-functions" $fpath)
   ```

**Fish** - add to `~/.config/fish/config.fish`:
   ```fish
   set -p fish_complete_path $HOME/.local/share/fish/vendor_completions.d
   ```


# How to package

If you are a package maintainer for a distribution:

- Build, generate completions, and install with `DESTDIR`, `PREFIX`, and
  `INTERCEPT_LIBDIR`:
  ```sh
  INTERCEPT_LIBDIR=lib64 cargo build --release
  target/release/generate-completions target/release/completions
  DESTDIR=$pkgdir PREFIX=/usr INTERCEPT_LIBDIR=lib64 ./scripts/install.sh
  ```

  `DESTDIR` is the staging root prepended to all install paths. `PREFIX` is
  the final on-system prefix (e.g. `/usr`). Do not combine them into
  `PREFIX` - the entry script embeds `PREFIX` as a literal runtime path,
  so it must match the installed system, not the build chroot.

- The preload library is only built on Unix: `libexec.so` on Linux and
  the BSDs, `libexec.dylib` on macOS. Windows builds only produce
  `bear-driver` and `bear-wrapper`. Consult
  `crates/intercept-preload/build.rs` for details.

- `bear-driver` locates its siblings using relative paths:
  `./bear-wrapper` and `../$INTERCEPT_LIBDIR/libexec.so`. The `bear` entry
  script in `$PREFIX/bin/` is the only artifact that uses an absolute path.

- The expected installation layout:
  ```
  $PREFIX/
  |-- bin/
  |   `-- bear                              (shell script)
  |-- libexec/
  |   `-- bear/
  |       |-- bin/
  |       |   |-- bear-driver
  |       |   `-- bear-wrapper
  |       `-- $INTERCEPT_LIBDIR/
  |           `-- libexec.so
  `-- share/
      |-- bash-completion/
      |   `-- completions/
      |       `-- bear                      (optional)
      |-- zsh/
      |   `-- site-functions/
      |       `-- _bear                     (optional)
      |-- fish/
      |   `-- vendor_completions.d/
      |       `-- bear.fish                 (optional)
      |-- elvish/
      |   `-- lib/
      |       `-- bear.elv                  (optional)
      |-- doc/
      |   `-- bear/
      |       |-- README.md
      |       `-- COPYING
      `-- man/
          `-- man1/
              `-- bear.1
  ```

- Common pitfalls:
  - If `INTERCEPT_LIBDIR` differs between the build and the install step,
    `bear-driver` looks for the preload library in the wrong place and
    preload interception silently fails at runtime. Set the same value
    for both, as shown above.
  - Build sandboxes that block `LD_PRELOAD` injection (for example
    Gentoo's Portage sandbox) will not observe anything under the
    default preload method; configure `intercept.mode: wrapper` for
    packages built or tested under such a sandbox (see `bear.yml` in
    `man/bear.1.md`).
  - After packaging, run the smoke test from step 5 above
    (`bear --version` and `bear -- true`) against the installed
    `$PREFIX/bin/bear`, not just the build directory, to confirm the
    installed layout resolves correctly.

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

To build and install Bear, run the following commands:

1. Clone the repository:
   ```bash
   git clone https://github.com/rizsotto/Bear.git
   cd bear
   ```

2. Build:
   ```bash
   cargo build --release
   ```

3. Install:
   ```bash
   sudo mkdir -p /usr/local/libexec/bear
   sudo mkdir -p /usr/local/man/man1
   sudo install -m 755 target/release/bear /usr/local/bin/
   sudo install -m 755 target/release/wrapper /usr/local/libexec/bear/
   sudo install -m 644 man/bear.1 /usr/local/man/man1/
   ```

To install the preload library, you need to determine the directory the dynamic
linker uses to resolve the `$LIB` symbol. You can find more information about
this in the `ld.so` man page (`man ld.so`).

   ```bash
   # For RedHat, Fedora, Arch based systems
   export INSTALL_LIBDIR=lib64
   
   # For Debian based systems
   export INSTALL_LIBDIR=lib/x86_64-linux-gnu
   ```

Then run the following commands:

   ```bash
   sudo mkdir -p /usr/local/libexec/bear/$INSTALL_LIBDIR
   sudo install -m 755 target/release/libexec.so /usr/local/libexec/bear/$INSTALL_LIBDIR/
   ```

# How to package

If you are a package maintainer for a distribution, there are a few extra
things you might want to know:

- The Bear executable contains hardcoded paths to the `wrapper` executable and
  the `libexec.so` shared library. If you change the location of these
  binaries, you also need to change the `bear/build.rs` file where these paths
  are set.
- Package the release build of this software. You can run the unit tests as
  part of the package build. Running the integration tests requires rebuilding
  the executables, so it is recommended to isolate the two steps as much as
  possible. Consult the CI configuration in `.github/workflows/build_rust.yml`
  for details.
- The preload mode is only enabled on Linux at the moment. Including
  `libexec.so` only makes sense on this OS. This might be extended to other
  operating systems in the future. Consult `intercept-preload/build.rs` for
  details.
- The preload library path contains a `$LIB` string, which the dynamic linker
  understands and resolves. This is useful in a multilib context. Consult the
  `ld.so` man page (`man ld.so`) for details.
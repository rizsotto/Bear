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
   cd Bear
   ```

2. Build:
   ```bash
   cargo build --release
   ```

3. Install:
   ```bash
   TARGET_DIR=/usr/local
   SHARE_DIR=$TARGET_DIR/share

   sudo mkdir -p $SHARE_DIR $SHARE_DIR/bear/bin
   sudo install -m 755 target/release/bear-driver $SHARE_DIR/bear/bin
   sudo install -m 755 target/release/bear-wrapper $SHARE_DIR/bear/bin
   sudo install -m 644 man/bear.1 $SHARE_DIR/man/man1

   cat > target/release/bear << EOF
   #!/bin/sh
   $SHARE_DIR/bear/bin/bear-driver "\$@"
   EOF
   sudo install -m 755 target/release/bear $TARGET_DIR/bin
   ```

To install the preload library, you need to determine the directory the dynamic
linker uses to resolve the `$LIB` symbol. You can find more information about
this in the `ld.so` man page (`man ld.so`).

   ```bash
   # For RedHat, Fedora, Arch based systems
   export LIBRARY_DIR=lib64
   # For Debian, Ubuntu based systems
   export LIBRARY_DIR=lib/x86_64-linux-gnu

   sudo mkdir -p $SHARE_DIR/bear/$LIBRARY_DIR
   sudo install -m 755 target/release/libexec.so $SHARE_DIR/bear/$LIBRARY_DIR
   ```

# How to package

If you are a package maintainer for a distribution, there are a few extra
things you might want to know:

- Package the release build of this software. You can run both the unit and
  integration tests as part of the package build. Consult the CI configuration
  in `.github/workflows/build_rust.yml` for details.
- The preload mode is only enabled on UNIX at the moment. Including
  `libexec.so` only makes sense on this OS. This might be extended to other
  operating systems in the future. Consult `intercept-preload/build.rs` for
  details.
- The final install should look like this. Where `bear` is a shell script,
  and the only program that uses absolute path to call `bear-driver`. The
  `bear-driver` is referencing `bear-wrapper` or `libexec.so` with relative
  path. (Using `./bear-wrapper` and `../$LIB/libexec.so` to reach these files.)
  This allows the installation process to choose the destination directory.

   ```bash
   $ tree /usr/local
   .
   ├── bin
   │   └── bear
   └── share
       ├── man
       │   └── man1
       │       └── bear.1
       └── bear
           ├── lib64
           │   └── libexec.so
           └── bin
               ├── bear-driver
               └── bear-wrapper
   ```

- The preload library path contains a `$LIB` string, which the dynamic linker
  understands and resolves. This is useful in a multilib context. Consult the
  `ld.so` man page (`man ld.so`) for details.

How to build
============

Bear is now implemented in Rust and can be built and installed using the Rust toolchain.

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
   git clone https://github.com/your-repo/bear.git
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

To install the preload library, you need to establish what the dynamic linker expects
to resolve the `$LIB` symbol. (Read `man ld.so` to get more about this.)

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

## OS-specific Notes

### Fedora/Red Hat-based systems
Install the Rust toolchain using the system package manager:
```bash
dnf install rust cargo
```

### Debian/Ubuntu-based systems
Install the Rust toolchain using the system package manager:
```bash
apt-get install rustc cargo
```

### macOS
Install Rust using [Homebrew](https://brew.sh/):
```bash
brew install rust
```

### Windows
Install Rust using [rustup](https://rustup.rs/).

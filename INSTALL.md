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

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Install the binary:
   ```bash
   cargo install --path .
   ```

This will install the `bear` binary to your Cargo bin directory (usually `~/.cargo/bin`).

## Running Tests

To run the tests, use:
```bash
cargo test
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

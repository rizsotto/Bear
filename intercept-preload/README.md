# Dynamic library for Bear interception

This crate provides a dynamic library for Unix systems that can be used with Bear for intercepting system calls via the `LD_PRELOAD` mechanism (or `DYLD_INSERT_LIBRARIES` on macOS).

## Overview

`libexec` is designed to work with the Bear compilation database generator. It intercepts system calls like `execve` to track command execution during builds.

The library is split into a C shim (`src/c/shim.c`) and Rust implementation (`src/implementation.rs`). This separation exists because:

1. Stable Rust cannot handle C variadic arguments (`execl` family)
2. On FreeBSD, libc functions may call each other internally — having all exported symbols in C call into Rust (which uses `dlsym(RTLD_NEXT, ...)`) avoids recursive interception issues

## Supported Platforms

- **Linux** — uses `LD_PRELOAD` and ELF version scripts for symbol visibility
- **macOS** — uses `DYLD_INSERT_LIBRARIES` and `-exported_symbols_list` for symbol visibility

On unsupported platforms, the build process will display a warning and skip library generation.

## Features

- Intercepts `exec` family calls, `posix_spawn`, `popen`, and `system`
- Automatically "doctors" child process environments to maintain interception across `exec` calls
- Reports intercepted executions to a TCP collector
- Platform capability detection at build time (only intercepts functions available on the host)

## Building

To build `libexec` in debug mode:

```bash
cargo build -p intercept-preload
```

For the release version:

```bash
cargo build -p intercept-preload --release
```

The resulting shared library will be in `target/debug/libexec.so` (or `.dylib` on macOS) and `target/release/libexec.so` respectively.
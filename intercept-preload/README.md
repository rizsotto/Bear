# Dynamic library for Bear interception

This crate provides a dynamic library (.so) for Linux that can be used with Bear for intercepting system calls via the `LD_PRELOAD` mechanism.

## Overview

`libexec` is designed to work with the Bear compilation database generator. It intercepts system calls like `execve` to track file access and command execution during builds.

## Features

- Intercepts system calls via `LD_PRELOAD` mechanism
- Builds a shared library (.so) for Linux systems
- Designed for transparent operation with build systems

## Building

To build `libexec` in debug mode:

```bash
cargo build -p intercept-preload
```

For the release version:

```bash
cargo build -p intercept-preload --release
```

The resulting shared library will be in `target/debug/libexec.so` or `target/release/libexec.so` respectively.

Note: The library will only build on Linux systems. On other platforms, the build process will display a warning and skip library generation.

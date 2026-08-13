# Summary

[Generate compile_commands.json for any C or C++ build](index.md)

- [Install Bear](installation.md)

# Tutorials

- [Getting started with Bear](tutorials/getting-started.md)
- [Build an autotools project under Bear](tutorials/autotools-tutorial.md)
- [Recover compile_commands.json from a build log](tutorials/compile-commands-from-a-build-log.md)

# Guides

- [Recipes](guides/recipes/index.md)
  - [Generate compile_commands.json for a Makefile project](guides/recipes/compile-commands-for-makefile.md)
  - [Generate compile_commands.json for a CMake project](guides/recipes/cmake.md)
  - [Capture a complete dry run for bear parse-sh](guides/recipes/dry-run-capture.md)
  - [Bear produces an empty compile_commands.json](guides/recipes/empty-compilation-database.md)
  - [Set up clangd for a project without CMake](guides/recipes/clangd-setup.md)
  - [clangd has no compile command for a header](guides/recipes/headers.md)
  - [Use Bear with ccache, distcc, or icecc](guides/recipes/ccache-distcc-icecc.md)
  - [Run Bear inside a Docker container](guides/recipes/docker.md)
  - [Generate compile_commands.json when cross-compiling](guides/recipes/cross-compilation.md)
  - [Intercept a 32-bit build on a 64-bit host](guides/recipes/multilib.md)
  - [Generate compile_commands.json for an STM32 or arm-none-eabi project](guides/recipes/embedded-arm.md)
  - [Use Bear with a vendor embedded toolchain](guides/recipes/vendor-embedded.md)
  - [Generate compile_commands.json for an Emscripten project](guides/recipes/emscripten.md)
  - [Generate compile_commands.json for a CUDA project](guides/recipes/cuda.md)
  - [Use Bear with Intel oneAPI compilers](guides/recipes/intel-oneapi.md)
  - [Use Bear with Cray and NVIDIA HPC compilers](guides/recipes/cray-hpc.md)
  - [Generate compile_commands.json for an MPI project](guides/recipes/mpi.md)
- [Troubleshooting](guides/troubleshooting.md)

# Reference

- [Command-line options](reference/command-line.md)
- [Configure Bear](reference/configuration.md)
- [Supported compilers](reference/supported-compilers.md)
- [Exit status](reference/exit-status.md)

# Understanding Bear

- [How Bear works](understanding/how-it-works.md)

# Platform notes

- [Bear on Linux, WSL2, and Docker](platforms/linux.md)
- [Bear on macOS](platforms/macos.md)
- [Bear on Windows](platforms/windows.md)
- [Bear on BSD](platforms/bsd.md)

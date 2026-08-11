<!-- Diataxis type: landing (navigation page, not one of the four types) -->

# Recipes

How to generate `compile_commands.json` for a specific build system,
toolchain, or environment, and what to do when the result is wrong.

- [Generate compile_commands.json for a Makefile project](compile-commands-for-makefile.md)
  - the main recipe: plain Make, autotools, incremental and parallel
  builds, recursive Makefiles, and recovering a database without
  running the build.
- [Generate compile_commands.json for a CMake project](cmake.md) - when
  CMake's own export is enough, and when it is not.
- [Bear produces an empty compile_commands.json](empty-compilation-database.md)
  - what to check when the file comes out empty or short.
- [Set up clangd for a project without CMake](clangd-setup.md) - point
  your editor at the database once Bear has written it.
- [clangd has no compile command for a header](headers.md) - when to let
  clangd infer one, and when to have Bear synthesize it.
- [Use Bear with ccache, distcc, or icecc](ccache-distcc-icecc.md) -
  keep a compiler launcher in the build while Bear still records the
  real compiler invocation.
- [Run Bear inside a Docker container](docker.md) - capture a build that
  runs inside a container.

## Toolchains

- [Generate compile_commands.json when cross-compiling](cross-compilation.md) -
  a target-prefixed GCC or Clang.
- [Intercept a 32-bit build on a 64-bit host](multilib.md) - a second
  preload library, for a build that runs 32-bit compilers or build
  tools.
- [Generate compile_commands.json for an STM32 or arm-none-eabi
  project](embedded-arm.md) - the bare-metal Arm GCC toolchain.
- [Use Bear with a vendor embedded toolchain](vendor-embedded.md) - QNX,
  Microchip XC8 / MPLAB X, and Texas Instruments.
- [Generate compile_commands.json for an Emscripten project](emscripten.md) -
  compiling to WebAssembly with `emcc`/`em++`.
- [Generate compile_commands.json for a CUDA project](cuda.md) - `nvcc`
  alongside a host compiler.
- [Use Bear with Intel oneAPI compilers](intel-oneapi.md) - the oneAPI
  and Classic Intel C/C++/Fortran drivers.
- [Use Bear with Cray and NVIDIA HPC compilers](cray-hpc.md) - HPE Cray's
  and NVIDIA's HPC SDK drivers.
- [Generate compile_commands.json for an MPI project](mpi.md) - the
  `mpicc`-style wrappers.

Related pages: [Getting started with Bear](../../tutorials/getting-started.md) for
the first run, [Troubleshooting](../troubleshooting.md) for a database
that came out wrong, and [How Bear works](../../understanding/how-it-works.md) for the
mechanism.

<!-- Diataxis type: how-to -->

# Generate compile_commands.json for an STM32 or arm-none-eabi project

Run the project's Makefile under Bear:

```sh
bear -- make
```

Cross-prefixed GNU Arm Embedded GCC (`arm-none-eabi-gcc`,
`arm-none-eabi-g++`) and Arm Compiler 6 (`armclang`, `armclang++`, the
compiler behind Keil MDK) are the two toolchains Bear recognizes for a
bare-metal Arm target; see [Supported
compilers](../../reference/supported-compilers.md) for the full name
table. `arm-none-eabi-gcc` is recognized through GCC's general
cross-compilation prefix support, covered in [Generate
compile_commands.json when cross-compiling](cross-compilation.md);
`armclang` has its own family instead.

## Where STM32CubeIDE's Makefile actually is

STM32CubeIDE does not build from a Makefile at the project root: it
generates one per build configuration, under a subdirectory named for
that configuration - `Debug/Makefile`, `Release/Makefile`, or a custom
configuration's own name - and that generated file is what the IDE's
"Build" button actually runs. Once a configuration has been built at
least once from the IDE, run Bear against that Makefile directly:

```sh
cd Debug
bear -- make
```

If the IDE already finished building that configuration, `make` there
has nothing left to do and Bear records nothing; force a full rebuild the
same way [Generate compile_commands.json for a Makefile
project](compile-commands-for-makefile.md#incremental-builds) recommends
for any other up-to-date build tree, for example `bear -- make clean
all`. A plain CubeMX-generated Makefile project (no IDE) writes its
Makefile at the project root instead of a per-configuration subdirectory,
but the same rule applies: run Bear against the Makefile that actually
compiles your sources, from clean if it is already up to date.

## When preload cannot inject into the compiler

Vendor Arm toolchains, including some STM32CubeIDE and Keil MDK
installations, still ship a 32-bit compiler driver binary even when
installed on a 64-bit host. Preload, the default interception method on
Linux, is built for the host's own ELF class, so injecting it into a
32-bit process fails with a "wrong ELF class" error from the dynamic
linker: the build still completes, but that invocation goes
unintercepted and its source file is silently missing from
`compile_commands.json`. [Generate compile_commands.json when
cross-compiling](cross-compilation.md#when-preload-cannot-reach-the-compiler)
covers this failure and its glibc-mismatch cousin in full, including the
`intercept.mode: wrapper` fix, which does not care about the target
binary's ELF class.

## What clangd needs for a bare-metal entry

The entry Bear records names the cross compiler itself
(`arm-none-eabi-gcc` or `armclang`) with the project's flags intact:
`-mcpu`, `-mthumb`, `--specs`, `-I` for the vendor HAL and CMSIS headers,
and any `-D` for the target MCU. Without
[`--query-driver`](https://clangd.llvm.org/config#compileflags) pointed at
that same cross compiler, clangd parses the entry with its own host
Clang, and host Clang rejects flags that only GCC's embedded target
support understands: `-mcpu=cortex-m4`, `-mthumb`, and
`--specs=nano.specs` each produce an "unknown argument" diagnostic
instead of the target and header search path they set on the real
compiler.

`--query-driver=/path/to/arm-none-eabi-gcc` fixes this by asking the real
driver for its target and built-in include paths instead of guessing from
host Clang's own, the same mechanism used for a wrapper's baked-in paths
on [Generate compile_commands.json for an MPI
project](mpi.md#what-actually-lands-in-the-entry). Without
that option, strip the GCC-specific flags in `.clangd` instead so clangd
stops rejecting the command line outright:

```yaml
CompileFlags:
  Remove: [-mcpu=*, -mthumb, --specs=*]
```

See [Set up clangd for a project without CMake](clangd-setup.md) for
pointing your editor at the database once it exists.

## Related pages

- [Generate compile_commands.json when cross-compiling](cross-compilation.md)
  for cross-prefix recognition and the preload/wrapper trade-off in
  general.
- [Use Bear with a vendor embedded toolchain](vendor-embedded.md) for
  QNX, Microchip XC8, and Texas Instruments' Arm toolchain.
- [Set up clangd for a project without CMake](clangd-setup.md).
- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table.
- [Recipes](index.md) for the other tasks.

<!-- Diataxis type: how-to -->

# Generate compile_commands.json for an STM32 or arm-none-eabi project

Build the project's Makefile under Bear:

```sh
bear -- make
```

STM32CubeIDE, CubeMX-generated projects, and most vendor Cortex-M SDKs
build through a generated Makefile even when the IDE hides that fact
behind a "Build" button, so the plain Make recipe applies unchanged: run
`bear -- make` from the directory holding that Makefile. See [Generate
compile_commands.json for a Makefile project](compile-commands-for-makefile.md)
for the general Make workflow (incremental builds, `-j`, recursive
subdirectories); this page only covers what differs for a bare-metal Arm
target.

## Two recognized toolchains

Bear recognizes two families of Arm bare-metal compiler:

- **Cross-prefixed GCC** - `arm-none-eabi-gcc`, `arm-none-eabi-g++`, and
  the rest of the GNU Arm Embedded naming pattern. This is the toolchain
  STM32CubeIDE and most vendor SDKs ship by default, and it is recognized
  through GCC's general cross-compilation prefix support; see [Generate
  compile_commands.json when cross-compiling](cross-compilation.md) for
  how that recognition works and what to do if preload cannot inject into
  it.
- **Arm Compiler 6** - `armclang`, `armclang++`, the compiler behind Keil
  MDK and some vendor SDK configurations. It maps to its own `armclang`
  family, not to `clang` or `gcc`.

Either name is recorded as invoked, arguments and all, the same as any
other compiler Bear recognizes.

## Getting a Makefile to build under Bear

Bear needs a build it can launch from a terminal. If your project only
exists as an IDE build configuration, generate or export a Makefile first
(CubeMX does this from its project settings), then run `bear -- make` in
the directory that holds it.

## What clangd needs for a bare-metal target

The entry Bear records names the cross compiler itself
(`arm-none-eabi-gcc` or `armclang`) with the project's flags intact:
`-mcpu`, `-mthumb`, `--specs`, `-I` for the vendor HAL and CMSIS headers,
and any `-D` for the target MCU. clangd reads the compiler name and those
flags from the entry to resolve the target and header search path the
same way the real compiler would, instead of falling back to the host's
own headers and triple. A Makefile that omits one of these flags for a
given file produces the same diagnostics gap under clangd that it would
under the real compiler; that is a build configuration issue, not
something Bear can add on its own. See [Set up clangd for a project
without CMake](clangd-setup.md) for pointing your editor at the database
once it exists.

## Related pages

- [Generate compile_commands.json when cross-compiling](cross-compilation.md)
  for cross-prefix recognition and the preload/wrapper trade-off.
- [Set up clangd for a project without CMake](clangd-setup.md).
- [Supported compilers](../supported-compilers.md) for the full
  recognized-name table.
- [Recipes](index.md) for the rest of the task pages.

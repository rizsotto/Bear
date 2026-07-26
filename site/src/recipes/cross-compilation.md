<!-- Diataxis type: how-to -->

# Generate compile_commands.json when cross-compiling

Run the cross build under Bear exactly as you would run it directly:

```sh
bear -- make
```

Bear records the cross compiler the same way it records a native one: the
executable path the build actually invoked, and every argument verbatim,
including a cross-compilation target prefix in the name
(`arm-none-eabi-gcc`, `aarch64-linux-gnu-g++`) and flags such as
`--sysroot` or a target-specific `-I`. Nothing about the build changes; the
two things worth knowing are how prefixed names get recognized, and one
limitation of the default interception method that shows up specifically
in cross builds.

## Prefixed and version-suffixed names

GCC, Clang, NVIDIA's `nvcc`, Flang, and Vala's `valac` are recognized
under a target prefix (`arm-linux-gnueabihf-gcc`, `aarch64-linux-gnu-clang++`),
a version suffix (`gcc-12`, `clang-15`), or both together
(`arm-linux-gnueabi-gcc-12`); every such spelling is parsed with its base
name's own flag rules, so the recorded entry looks exactly like it would
for the plain name. Arm Compiler 6 (`armclang`, `armclang++`) is
recognized with a version suffix but not a target prefix. This support
varies by family; it is not a blanket rule applied to every entry in [Supported
compilers](../supported-compilers.md), which is the source of truth for
the full name table - this page only covers the prefix/suffix behavior
that cross builds rely on.

Sysroot and include flags are not specific to cross-compilation at all:
Bear does not interpret or rewrite argument values (only `@file`
response-file expansion and a couple of environment-variable foldings are
optional exceptions, see `format.arguments` in the [`bear(1)` man
page][manpage]), so `--sysroot=/opt/arm-sdk/sysroot` and
`-I/opt/arm-sdk/sysroot/usr/include` land in the entry's `arguments`
exactly as the build passed them.

## When preload cannot reach the compiler

Preload is the default interception method on Linux and the BSDs, and it
works for a cross build the same way it works for a native one, with one
exception: the preload library is built for the host's ELF class. Many
vendor cross-toolchains still ship a 32-bit compiler driver binary even
on a 64-bit host (common for older embedded SDKs distributed as prebuilt
binaries); loading a 64-bit preload library into that 32-bit process
fails with a "wrong ELF class" error from the dynamic linker. The build
still completes, but that invocation is not intercepted, so its source
file is silently missing from `compile_commands.json`.

A related but different failure is a glibc symbol-version mismatch, which
[Troubleshooting](../troubleshooting.md#glibc-version-errors-in-cross-compilation)
covers in full, including the commands to compare glibc versions and the
fix (link a Bear build against a glibc no newer than the SDK's).

Either failure means preload cannot inject into that particular compiler
process. Switch to wrapper mode instead, since it substitutes the
compiler executable rather than injecting into it, so it does not care
about the target binary's ELF class or its glibc:

```yaml
schema: "4.2"
intercept:
  mode: wrapper
```

Wrapper mode needs the build to discover the wrapper as the compiler, so
a "configure" step that detects compilers on its own has to run under
Bear too; see [Configure Bear](../configuration.md) for how to set the
mode and [How Bear works](../how-it-works.md#two-ways-to-capture-a-real-build)
for what each method can and cannot reach.

## Related pages

- [Generate compile_commands.json for an STM32 or arm-none-eabi
  project](embedded-arm.md) for the Arm bare-metal case.
- [Use Bear with Texas Instruments compilers](ti-compilers.md) and [Use
  Bear with Microchip XC8](microchip-xc8.md) for two specific vendor
  toolchains.
- [Supported compilers](../supported-compilers.md) for the full name
  table and the `compilers:` override.
- [Generate compile_commands.json for a Makefile project](compile-commands-for-makefile.md)
  for the general Make workflow this page builds on.
- [Recipes](index.md) for the rest of the task pages.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

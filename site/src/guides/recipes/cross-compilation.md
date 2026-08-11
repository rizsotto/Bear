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
`--sysroot` or a target-specific `-I`. Bear does not interpret or rewrite
argument values (only `@file` response-file expansion and a couple of
environment-variable foldings are optional exceptions, see
`format.arguments` in [Configure Bear](../../reference/configuration.md)),
so a value like `--sysroot=/opt/arm-sdk/sysroot` lands in the entry's
`arguments` exactly as the build passed it. Nothing about the build
changes; what is worth knowing is which names get recognized as
cross-compiled, and one limitation of the default interception method
that shows up specifically in cross builds.

## Prefixed and version-suffixed names

GCC, Clang, NVIDIA's `nvcc`, Flang, and Vala's `valac` are recognized
under a cross-compilation target prefix, a version suffix, or both
together; every such spelling is parsed with its base name's own flag
rules. This support varies by family - it is not a blanket rule applied
to every entry - so [Cross-compiler prefixes and version
suffixes](../../reference/supported-compilers.md#cross-compiler-prefixes-and-version-suffixes)
is the source of truth for which families it covers and how the pattern
works; this page only covers the cross-build-specific failure below.

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
Bear too; see [Configure Bear](../../reference/configuration.md) for how to set the
mode and [How Bear works](../../understanding/how-it-works.md#two-ways-to-capture-a-real-build)
for what each method can and cannot reach.

## Related pages

- [Generate compile_commands.json for an STM32 or arm-none-eabi
  project](embedded-arm.md) for the Arm bare-metal case.
- [Use Bear with a vendor embedded toolchain](vendor-embedded.md) for
  QNX, Microchip XC8, and Texas Instruments.
- [Supported compilers](../../reference/supported-compilers.md) for the full name
  table and the `compilers:` override.
- [Generate compile_commands.json for a Makefile project](compile-commands-for-makefile.md)
  for the general Make workflow.
- [Recipes](index.md) for the other tasks.

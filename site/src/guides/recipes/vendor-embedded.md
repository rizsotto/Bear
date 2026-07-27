<!-- Diataxis type: how-to -->

# Use Bear with a vendor embedded toolchain

Run the project's Makefile under Bear exactly as you already build it:

```sh
bear -- make
```

This covers three vendor toolchains: QNX Momentics, Microchip's MPLAB X
(XC8), and Texas Instruments' Clang-based Arm compiler. Each builds
through a Makefile (MPLAB X generates one), and Bear needs nothing
toolchain-specific once that Makefile runs under it. What differs between
them is which executable name gets recognized as what; the full name
table, `as:` ids, and prefix/suffix rules for every family are on
[Supported compilers](../../reference/supported-compilers.md) and are not
repeated here.

## QNX

Bear recognizes `qcc` and `q++`, QNX Neutrino's compiler drivers, under
the `qnx` family, parsed with GCC's flag rules (QNX 8 ships a GCC
12.2-based toolchain). This is a fixed pair of names: unlike GCC or
Clang, `qcc`/`q++` are not recognized under a cross-compilation prefix or
a version suffix.

A QNX SDP installation also carries a set of `ntoARCH`-prefixed GCC
binaries underneath the driver (`ntoaarch64-gcc`, `ntox86_64-gcc`, and
similar, one per target architecture). These are recognized too, but as
`gcc`, not `qnx`: GCC's cross-compilation prefix pattern matches any name
ending in `-gcc`, and QNX's target names happen to fit that shape. A
Makefile that calls one of these directly, rather than through `qcc`,
still gets an entry, parsed with GCC's own flag rules instead of QNX's.

QNX's `-V` flag picks the target/compiler variant on the command line
(`-Vgcc_ntoaarch64le`); the `qnx` family models it as a single token that
is never split and never swallows a following source file, matching both
the attached form and the bare `-V` (which lists available variants).

## Microchip XC8 / MPLAB X

Bear recognizes both `xc8-cc` (the GCC-styled driver name used from XC8
v2.30 onward) and the legacy `xc8` name, and records both under the
`gcc` family, not an XC8-specific id: XC8's command-line syntax is
GCC-styled, so Bear parses it with GCC's own flag rules.

XC16 and XC32 need no entry of their own: their driver names, `xc16-gcc`
and `xc32-gcc`, already match the general `<prefix>-gcc` cross-compilation
pattern GCC is recognized under (see [Generate compile_commands.json when
cross-compiling](cross-compilation.md)). XC8 needed its own table row
because `xc8-cc` and `xc8` do not follow that prefix pattern.

MPLAB X generates a Makefile for every build configuration and uses it as
the project's real build entry point, the same way its own "Build" button
does internally. Run `bear -- make` from the project directory that holds
that generated Makefile.

## Texas Instruments

Bear recognizes `tiarmclang`, TI's Clang-based compiler for Arm targets,
under the `clang` family (not a TI-specific id, and not `armclang`, which
names Arm Ltd.'s own Compiler 6 instead).

TI's earlier, non-Clang Code Generation Tools - the classic drivers for
C2000, C6000, MSP430, and the pre-Clang Arm compiler (`armcl`, `cl2000`,
`cl430`, `cl6x`) - are not in Bear's recognition table at all. An
invocation of one of these produces no entry: not a wrong one, none.

A [`compilers:` entry](../../reference/supported-compilers.md#as-and-ignore-hints)
pointing at the executable's path can force one of these to be parsed
with, for example, `as: gcc`, but that only helps to the extent its flag
dialect actually matches GCC's. It does not for the option that matters
most for the recorded entry: these tools spell an output file
`--output_file=main.obj`, not GCC's `-o main.obj`, so the `output` field
GCC's rules populate from `-o` comes out empty, and every other flag the
two dialects spell differently is misparsed the same way. The other
option is filing an issue against the project; see [what is not
recognized](../../reference/supported-compilers.md#what-is-not-recognized).

## Related pages

- [Generate compile_commands.json when cross-compiling](cross-compilation.md)
  for cross-compilation prefix and version-suffix recognition in general.
- [Generate compile_commands.json for an STM32 or arm-none-eabi
  project](embedded-arm.md) for the Arm bare-metal GCC / Arm Compiler 6
  case.
- [Supported compilers](../../reference/supported-compilers.md) for the
  full recognized-name table and the `compilers:` override.
- [Generate compile_commands.json for a Makefile
  project](compile-commands-for-makefile.md) for the general Make
  workflow these toolchains build on.
- [Recipes](index.md) for the rest of the task pages.

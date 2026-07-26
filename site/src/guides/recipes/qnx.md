<!-- Diataxis type: how-to -->

# Use Bear with the QNX qcc compiler

Run the QNX Momentics build under Bear exactly as you already invoke it:

```sh
bear -- make
```

Bear recognizes `qcc` and `q++`, QNX Neutrino's compiler drivers, and
parses their command line with GCC's flag rules (QNX 8 ships a GCC
12.2-based toolchain). Nothing about the Makefile changes; the recorded
entry carries the driver's path and arguments exactly as invoked, the
same as any GCC build.

## What is recognized, and under which id

`qcc` and `q++` are recognized under the `qnx` family (`as: qnx` in the
configuration). This is a fixed pair of names: unlike GCC or Clang, `qcc`
and `q++` are not recognized under a cross-compilation target prefix or a
version suffix, since QNX ships them under these two names only.

A QNX SDP installation also carries a set of `ntoARCH`-prefixed GCC
binaries underneath the driver (`ntoaarch64-gcc`, `ntox86_64-gcc`, and
similar, one per target architecture). These are recognized too, but as
`gcc`, not as `qnx`: GCC's cross-compilation prefix pattern matches any
name ending in `-gcc`, and QNX's target names happen to fit that shape.
A Makefile that calls one of these directly, rather than through `qcc`,
still gets an entry, parsed with GCC's own flag rules rather than QNX's.
See [Supported compilers](../../reference/supported-compilers.md) for the full name
table and how prefix and suffix recognition work in general.

## The `-V` variant selector

QNX's `-V` flag picks the target/compiler variant on the command line
(`-Vgcc_ntoaarch64le`), and the `qnx` family models it as a single token
that is never split and never swallows a following source file, matching
both the attached form and the bare `-V` (which lists available
variants). A build that compiles several variants by invoking `qcc -V...`
once per architecture records one entry per invocation, each with its own
`-V` value intact in `arguments`.

## Recursive and multi-variant builds

QNX Momentics projects commonly build through recursive Makefiles,
often compiling more than one CPU variant in a single pass. Bear needs no
extra flags for this: every intercepted `qcc`/`q++` invocation carries
its own working directory, so entries come out correctly scoped
regardless of how deep the recursion goes or how many variants the build
produces in one run. See [Generate compile_commands.json for a Makefile
project](compile-commands-for-makefile.md#recursive-makefiles) for the
general recursion behavior this relies on.

## Related pages

- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table and the `compilers:` override for a path Bear
  gets wrong.
- [Generate compile_commands.json for a Makefile
  project](compile-commands-for-makefile.md) for the general Make
  workflow this page builds on.
- [Generate compile_commands.json when
  cross-compiling](cross-compilation.md) for cross-compilation prefix
  and version-suffix recognition in general.
- [Recipes](index.md) for the rest of the task pages.

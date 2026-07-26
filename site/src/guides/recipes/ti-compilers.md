<!-- Diataxis type: how-to -->

# Use Bear with Texas Instruments compilers

Build the project's Makefile under Bear:

```sh
bear -- make
```

Bear recognizes `tiarmclang`, TI's Clang-based compiler for Arm targets,
and records its invocations under the `clang` family (not a
TI-specific id, and not `armclang`, which names Arm Ltd.'s own Compiler
6 instead).

## Older TI compilers are not recognized

TI's earlier, non-Clang Code Generation Tools - the classic drivers for
C2000, C6000, MSP430, and the pre-Clang Arm compiler (`armcl` and
similar) - are not in Bear's recognition table at all. An invocation of
one of these produces no entry: not a wrong one, none.

Two ways forward:

- Add a `compilers:` entry pointing at the executable's path with an
  `as:` hint, for example `as: gcc`, so Bear parses its command line
  instead of ignoring it. The entry that comes out names the file Bear
  found among the arguments and the full recorded command line; fields
  that depend on GCC's own flag spelling (such as the `output` field,
  populated only when the command uses `-o`) are only as accurate as
  the hinted family's flag rules match the tool's real syntax.
- Or file an issue against the project. A toolchain no `as:` value fits
  is a gap in Bear's compiler definitions rather than a configuration
  problem; see [Supported
  compilers](../../reference/supported-compilers.md#what-is-not-recognized).

See [Supported compilers](../../reference/supported-compilers.md#as-and-ignore-hints)
for the full `as`/`ignore` explanation and the complete list of accepted
`as` values, and the `compilers` section of the [`bear(1)` man
page][manpage] for the configuration key itself.

## Related pages

- [Generate compile_commands.json when cross-compiling](cross-compilation.md)
  for how cross-compiler names are recognized in general.
- [Generate compile_commands.json for an STM32 or arm-none-eabi
  project](embedded-arm.md) for the other Arm bare-metal toolchains Bear
  recognizes.
- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table.
- [Recipes](index.md) for the rest of the task pages.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

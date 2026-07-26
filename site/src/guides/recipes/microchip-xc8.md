<!-- Diataxis type: how-to -->

# Use Bear with Microchip XC8

Build the project's generated Makefile under Bear:

```sh
bear -- make
```

Bear recognizes both `xc8-cc` (the GCC-styled driver name used from XC8
v2.30 onward) and the legacy `xc8` driver name. Both are recorded under
the `gcc` family, not an XC8-specific id: XC8's command-line syntax is
GCC-styled, so Bear parses it with GCC's own flag rules.

Microchip's other compiler families, XC16 and XC32, do not need a
listing of their own: their driver names, `xc16-gcc` and `xc32-gcc`,
already match the general `<prefix>-gcc` cross-compilation pattern GCC is
recognized under (see [Generate compile_commands.json when
cross-compiling](cross-compilation.md)). XC8 needed its own entry because
`xc8-cc` and `xc8` do not follow that prefix pattern.

## MPLAB X projects

MPLAB X generates a Makefile for every build configuration and uses it
as the project's real build entry point, the same way its own "Build"
button does internally. Run `bear -- make` from the project directory
that holds that generated Makefile; see [Generate compile_commands.json
for a Makefile project](compile-commands-for-makefile.md) for the general
Make workflow (incremental builds, `-j`, recursive subdirectories) this
page builds on.

## Related pages

- [Generate compile_commands.json when cross-compiling](cross-compilation.md)
  for how cross-prefixed GCC names are recognized in general.
- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table and the `compilers:` override.
- [Recipes](index.md) for the rest of the task pages.

<!-- Diataxis type: how-to -->

# Use Bear with Intel oneAPI compilers

Source the oneAPI environment script for the current shell, then run the
build under Bear exactly as you would for any other compiler:

```sh
source /opt/intel/oneapi/setvars.sh
bear -- make
```

`setvars.sh` is what puts `icx`, `icpx`, `ifx`, and the rest of the oneAPI
drivers on `PATH`. Bear itself does nothing toolchain-specific at that
point: it watches whatever executable the build actually runs, the same
as for GCC or Clang. Skipping the `source` step is the most common
reason a build "under Bear" still records the system compiler: the shell
running `make` never had the oneAPI drivers on `PATH` to begin with, so
`bear -- make` and a plain `make` invoke exactly the same programs.

## Which family each driver is recognized as

Bear recognizes the oneAPI and Classic C/C++ drivers under one family
id, `intel_cc`:

- `icx`, `icpx` (the current oneAPI drivers)
- `icc`, `icpc` (the older Classic drivers)

Both parse with the same Intel-specific flag table (things like
`-qopenmp`, `-ipo`, and `-fp-model`), and `bear semantic
--print-compilers` lists both pairs as `as: intel_cc`.

The Fortran drivers, `ifort` and `ifx`, are a separate family,
`intel_fortran`.

Intel's MPI wrappers (`mpiicc`, `mpiifx`, and the rest) are recognized
too, and they reuse these same two families rather than the generic MPI
wrapper id. See [Generate compile_commands.json for an MPI
project](mpi.md) for the full mapping, why the split exists, and what
gets recorded when a build calls one of these wrappers instead of the
driver directly.

The full recognized-name table, including every other vendor built on
GCC or Clang, is on [Supported compilers](../../reference/supported-compilers.md);
this page only calls out the Intel-specific mappings.

## If a driver is not recognized

A build that calls the compiler through a renamed copy or a nonstandard
path (a custom install, a CI image that symlinks `icx` to something
else) needs a hint in the configuration file:

```yaml
schema: "4.2"
compilers:
  - path: /opt/custom/bin/icx-wrapper
    as: intel_cc
```

Use `as: intel_fortran` for a renamed Fortran driver. The accepted `as`
values are exactly the family ids `bear semantic --print-compilers`
shows; see the `compilers` section of the [`bear(1)` man
page][manpage] for the full key.

## Related pages

- [Generate compile_commands.json for an MPI project](mpi.md) for the
  Intel MPI wrappers specifically.
- [Supported compilers](../../reference/supported-compilers.md) for the complete
  recognized-name table and the ambiguous-name rules.
- [Configure Bear](../../reference/configuration.md) for the `compilers:` section.
- [Recipes](index.md) for the rest of the task pages.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

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

`icx`/`icpx` (current oneAPI) and `icc`/`icpc` (the older Classic
drivers) share one family id, `intel_cc`, and one Intel-specific flag
table (`-qopenmp`, `-ipo`, `-fp-model`, and the rest). The Fortran
drivers, `ifort` and `ifx`, are a separate family, `intel_fortran`. See
[Supported compilers](../../reference/supported-compilers.md) for the
full recognized-name table; `bear semantic --print-compilers` prints the
same mapping for the version you have installed.

Intel's MPI wrappers (`mpiicc`, `mpiifx`, and the rest) reuse these same
two families rather than a generic MPI wrapper id. See [Generate
compile_commands.json for an MPI project](mpi.md) for that mapping and
why the split exists.

## If a driver is not recognized

A build that calls the compiler through a renamed copy or a nonstandard
path (a custom install, a CI image that symlinks `icx` to something else)
needs a `compilers:` hint:

```yaml
schema: "4.2"
compilers:
  - path: /opt/custom/bin/icx-wrapper
    as: intel_cc
```

Use `as: intel_fortran` for a renamed Fortran driver. See [`as` and
`ignore` hints](../../reference/supported-compilers.md#as-and-ignore-hints)
for the accepted values, and the `compilers` section of [Configure
Bear](../../reference/configuration.md) for the key itself.

## Related pages

- [Generate compile_commands.json for an MPI project](mpi.md) for the
  Intel MPI wrappers specifically.
- [Supported compilers](../../reference/supported-compilers.md) for the complete
  recognized-name table and the ambiguous-name rules.
- [Configure Bear](../../reference/configuration.md) for the `compilers:` section.
- [Recipes](index.md) for the other tasks.

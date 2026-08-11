<!-- Diataxis type: how-to -->

# Use Bear with Cray and NVIDIA HPC compilers

Load the programming environment module for the current shell, then run
the build under Bear:

```sh
module load PrgEnv-cray
bear -- make
```

On an HPE Cray system the toolchain is selected by which module is
loaded, not by which package is installed: `module load` rewrites `PATH`
(and other environment variables) for the shell that runs it. Once the
right compiler is on `PATH`, Bear needs nothing else; it watches
whatever the build executes, the same as anywhere else. The same applies
to an NVIDIA HPC SDK module (`module load nvhpc`) on a cluster that
provides one.

## Recognized drivers

Cray's own compiler drivers - `craycc`, `crayCC`, `craycxx` (`cray_cc`,
built on Clang, with a few Cray-specific extensions such as `-fcray-*`)
and `crayftn`/`ftn` (`cray_fortran`) - and the NVIDIA HPC SDK's `nvc`,
`nvc++`, `nvfortran` (`nvidia_hpc`) are all recognized directly. The
legacy PGI names, `pgcc`, `pgc++`, `pgfortran`, map to the same
`nvidia_hpc` family and its flag table (`-Mvect`, `-acc`, `-gpu`, and the
rest), since the PGI compiler became the NVIDIA HPC SDK. See [Supported
compilers](../../reference/supported-compilers.md) for the full name
table.

## `cc`, `CC`, and `ftn`: the PrgEnv wrapper names

`cc` and `CC` are ambiguous on a Cray system - either can front CCE, GCC,
or another vendor's compiler depending on the loaded programming
environment module - so neither is in Bear's static table; Bear
classifies them by probing `--version` once and caching the result, the
mechanism [Supported
compilers](../../reference/supported-compilers.md#ambiguous-names)
documents in general. The probe answers `gcc` or `clang` only, never
`cray_cc`, so add a `compilers:` entry when you need the family pinned
exactly, or when the probe cannot identify the compiler at all.

`ftn`, by contrast, is recognized directly as Cray Fortran (`as:
cray_fortran`) regardless of which programming environment module is
loaded. If a loaded module makes `ftn` front a different Fortran
compiler on your system, override it the same way you would any
misclassified compiler:

```yaml
schema: "4.2"
compilers:
  - path: /opt/cray/pe/craype/default/bin/ftn
    as: intel_fortran
```

## Related pages

- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table and the ambiguous-name probe.
- [Generate compile_commands.json for an MPI project](mpi.md) if the
  build also calls `mpicc`/`mpifort`-style wrappers directly instead of
  the `cc`/`CC`/`ftn` drivers.
- [Configure Bear](../../reference/configuration.md) for the `compilers:` section.
- [Recipes](index.md) for the other tasks.

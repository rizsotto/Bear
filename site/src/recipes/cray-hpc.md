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

## Cray compiler drivers

Bear recognizes the Cray Compiling Environment's own executable names
directly:

- `craycc`, `crayCC`, `craycxx` are Cray C/C++, family id `cray_cc`
  (built on Clang, so it parses with Clang's flag table plus a few
  Cray-specific extensions such as `-fcray-*`).
- `crayftn`, and also the generic `ftn` name, are Cray Fortran, family
  id `cray_fortran`.

## NVIDIA HPC SDK, including the legacy PGI names

`nvc`, `nvc++`, and `nvfortran` are the current NVIDIA HPC SDK drivers,
family id `nvidia_hpc`. The older PGI names, `pgcc`, `pgc++`, and
`pgfortran`, map to the same `nvidia_hpc` family: they take the same
flag table (`-Mvect`, `-acc`, `-gpu`, and the rest of the `-M*`/`-gpu`
surface), since the PGI compiler became the NVIDIA HPC SDK.

## `cc`, `CC`, and `ftn`: the PrgEnv wrapper names

The generic Cray PrgEnv wrapper names are not all handled the same way.
`cc` and `CC` are ambiguous: on a Cray system either one can front CCE,
GCC, or another vendor's compiler depending on the loaded programming
environment module, so neither name is in Bear's static recognition
table at all. Bear classifies an invocation of `cc` or `CC` by running
it once with `--version` and caching the result. The probe answers `gcc`
or `clang` only, never `cray_cc`, so add a `compilers:` entry when you
need the family pinned exactly, and when the probe cannot identify the
compiler at all. This is the same mechanism, and the same `CC` special
case, documented on [Supported
compilers](../supported-compilers.md#ambiguous-names).

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

- [Supported compilers](../supported-compilers.md) for the full
  recognized-name table and the ambiguous-name probe.
- [Generate compile_commands.json for an MPI project](mpi.md) if the
  build also calls `mpicc`/`mpifort`-style wrappers directly instead of
  the `cc`/`CC`/`ftn` drivers.
- [Configure Bear](../configuration.md) for the `compilers:` section.
- [Recipes](index.md) for the rest of the task pages.

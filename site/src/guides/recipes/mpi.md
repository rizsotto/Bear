<!-- Diataxis type: how-to -->

# Generate compile_commands.json for an MPI project

Run the build under Bear exactly as you would without MPI:

```sh
bear -- make
```

A build that compiles through `mpicc`, `mpicxx`, `mpifort`, or one of
the Intel MPI wrappers needs no special configuration: Bear recognizes
all of them and records the wrapper invocation itself, so
`compile_commands.json` ends up with entries such as `mpicc -c main.c -o
main.o`, not the underlying compiler command the wrapper builds
internally.

## Which family each wrapper is recognized as

Open MPI's and MPICH's wrappers share one family id, `mpi`: `mpicc`,
`mpicxx`, `mpic++`, `mpiCC`, `mpifort`, `mpif77`, `mpif90`. They parse
with the GCC flag table, since these wrappers front a gcc-compatible
driver.

Intel MPI's wrappers are recognized too, but as extra names on the
Intel compiler families rather than the `mpi` id: `mpiicc`, `mpiicpc`,
`mpiicx`, `mpiicpx` are `intel_cc`, and `mpiifort`, `mpiifx` are
`intel_fortran`. That split exists because `mpiicc` is `icc`/`icx` with
MPI's paths baked in, and Intel's flag table has arity rules (for
example `-debug` takes a following argument) that the GCC table would
mis-parse. See [Use Bear with Intel oneAPI
compilers](intel-oneapi.md) for those two families in general.

## The wrapper is recorded as invoked, not expanded

An MPI wrapper is a different kind of wrapper than ccache, distcc, or
sccache. Those launchers carry the real compiler as one of their own
arguments (`ccache gcc -c main.c`), so Bear can drop the launcher's name
and record the argument that follows it. An MPI wrapper does not carry
its underlying compiler on the command line at all: the compiler and the
extra `-I`/`-L` flags for MPI's headers and libraries are baked into the
wrapper script at MPI-installation time, and the only way to learn them
is to run the wrapper itself (`mpicc -show` on MPICH, `mpicc -showme` on
Open MPI).

Bear does not run the wrapper to find that out, so it records the wrapper
invocation verbatim instead of expanding it. Given

```sh
mpicc -c main.c -o main.o
```

where `mpicc` is a script that execs `gcc` with `-I/opt/mpi/include
-L/opt/mpi/lib` prepended, the entry Bear writes is

```json
[
  {
    "file": "main.c",
    "arguments": ["mpicc", "-c", "main.c", "-o", "main.o"],
    "directory": "/path/to/project",
    "output": "main.o"
  }
]
```

`arguments` holds the command as the build wrote it. It does not contain
`gcc`, and it does not contain the wrapper's baked-in `-I`/`-L` flags. In
preload mode Bear does intercept the compiler the wrapper execs, but the
default duplicate filter (`directory` and `file`) keeps only the wrapper's
entry, which comes first; that second invocation surfaces as its own entry
only if you add `arguments` to `duplicates.match_on`.

Clang tooling that needs those flags to resolve MPI headers should point
at the wrapper directly; clangd does this with `--query-driver`, which
asks the wrapper for its implicit include paths without Bear needing to
run it. See [Set up clangd for a project without
CMake](clangd-setup.md) for pointing an editor at the database this page
produces.

## Wrapper-info options produce no entry

`mpicc -showme`, `-show`, `-compile_info`, and `-link_info` print the
underlying compiler command and exit without compiling anything, so none
of them produce a database entry. A build step that only queries the
wrapper this way (for example a `configure` probe) is silently skipped,
the same as any other non-compiling command.

MPICH's compiler-override options, `-cc=`, `-cxx=`, and `-fc=` (with or
without the `=`), are recognized as driver options that take a value, so
`-cc gcc` does not get misread as compiling a source file named `gcc`.

## Related pages

- [Use Bear with Intel oneAPI compilers](intel-oneapi.md) for the
  `intel_cc`/`intel_fortran` families the Intel MPI wrappers reuse.
- [Use Bear with ccache, distcc, or icecc](ccache-distcc-icecc.md) for
  the launcher case this page contrasts with.
- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table.
- [Recipes](index.md) for the rest of the task pages.

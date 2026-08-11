<!-- Diataxis type: how-to -->

# Generate compile_commands.json for an MPI project

Run the build under Bear exactly as you would without MPI:

```sh
bear -- make
```

A build that compiles through `mpicc`, `mpicxx`, `mpifort`, or one of the
Intel MPI wrappers needs no special configuration: Bear recognizes all of
them and records the wrapper invocation itself. See [MPI compiler
wrappers](../../reference/supported-compilers.md#mpi-compiler-wrappers)
and [Supported compilers](../../reference/supported-compilers.md) for the
full name table and the wrapper-is-recorded-as-invoked contract in
general. This page covers what that contract means in practice: what
actually lands in the recorded entry, and the one place the database can
differ from what a reader expects.

## Open MPI/MPICH vs. Intel

Open MPI's and MPICH's wrappers share one family id, `mpi`, and parse
with the GCC flag table. Intel's wrappers (`mpiicc`, `mpiifort`, and the
rest) are recognized too, but as extra names on the `intel_cc` and
`intel_fortran` families instead of the generic `mpi` id: `mpiicc` is
`icc`/`icx` with MPI's paths baked in, and Intel's flag table has arity
rules (for example `-debug` takes a following argument) that the GCC
table would mis-parse. See [Use Bear with Intel oneAPI
compilers](intel-oneapi.md) for those two families in general.

## What actually lands in the entry

Given

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

`arguments` holds the command exactly as the build wrote it: no `gcc`,
and none of the wrapper's baked-in `-I`/`-L` flags, since Bear does not
run the wrapper to learn them. Clang tooling that needs those flags to
resolve MPI headers should point at the wrapper directly; clangd does
this with `--query-driver`. See [Set up clangd for a project without
CMake](clangd-setup.md) for pointing an editor at the database once it
exists.

## The compiler the wrapper execs is usually not a second entry

In preload mode Bear does intercept the compiler the wrapper execs
internally, not just the wrapper itself, but that rarely shows up as a
second entry in `compile_commands.json`: the default duplicate filter
(`directory` and `file`) treats the two invocations as the same
compilation and keeps whichever comes first, the wrapper's. That
lower-level invocation surfaces as its own entry only if you widen
`duplicates.match_on` to include `arguments` (see [Configure
Bear](../../reference/configuration.md#duplicates)); ordinarily that is
not useful, since it names an intermediate compiler call rather than the
one your build system actually asked for.

MPICH's compiler-override options, `-cc=`, `-cxx=`, and `-fc=`, are also
accepted without the `=` (`-cc gcc`); either form is recognized as a
driver option that takes a value, so it never gets misread as compiling a
source file named `gcc`.

## Related pages

- [Use Bear with Intel oneAPI compilers](intel-oneapi.md) for the
  `intel_cc`/`intel_fortran` families the Intel MPI wrappers reuse.
- [Use Bear with ccache, distcc, or icecc](ccache-distcc-icecc.md) for
  the launcher case, where Bear records the wrapped compiler instead.
- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table.
- [Recipes](index.md) for the other tasks.

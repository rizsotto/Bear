---
title: Recognize MPI compiler wrappers
status: in-progress
---

## Intent

When a build compiles through MPI wrapper commands (Open MPI, MPICH,
Intel MPI), Bear records those compilations in the compilation database
without any configuration. The recorded compiler is the wrapper itself,
exactly as the build invoked it.

## Acceptance criteria

- An execution of `mpicc`, `mpicxx`, `mpic++`, `mpiCC`, `mpifort`,
  `mpif77`, or `mpif90` with a source file yields a database entry.
- The recorded arguments are the wrapper invocation as executed; the
  wrapper is not expanded to the underlying compiler command.
- Intel MPI wrappers (`mpiicc`, `mpiicpc`, `mpiicx`, `mpiicpx`,
  `mpiifort`, `mpiifx`) are recognized with Intel flag semantics.
- Wrapper-info invocations (`mpicc -showme`, `mpicc -show`,
  `mpicc -compile_info`, and the like) yield no entry.
- The wrapper's compiler-override options that carry a value (such as
  MPICH's `-cc=gcc`) never swallow a following source file.
- MPI launchers (`mpirun`, `mpiexec`) are not recognized as compilers;
  they execute programs, they do not compile.

## Testing

Given an event file with an `mpicc -c hello.c` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`
> whose arguments start with `mpicc`.

Given an event file with an `mpicc -showme` (or `mpicc -show`,
`mpicc -compile_info`) execution:

> When `bear semantic` runs,
> then the database contains no entry for that execution.

Given an event file with an `mpicc -cc=gcc -c hello.c` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`
> and the `-cc=gcc` token is retained in the arguments.

Given an event file with an `mpicc -c hello.c` event followed by the
wrapper's child compiler event `gcc -c hello.c` (both processes are
intercepted in preload mode):

> When `bear semantic` runs with default duplicate detection,
> then exactly one entry survives for `hello.c`,
> and it records the wrapper invocation (the wrapper event comes first
> in the event stream, so it wins under first-seen duplicate detection).

## Notes

- The wrapper invocation is recorded verbatim. Clang tooling users who
  need the wrapper's baked-in include paths can point their tool at the
  wrapper (e.g. clangd's `--query-driver`).
- The launcher names `mpirun` and `mpiexec` stay unrecognized on
  purpose; there is no acceptance path for them.

## Rationale

- [MPI wrappers as compilers](../rationale/mpi-wrappers-as-compilers.md) -
  why the wrapper is recorded verbatim instead of expanded to the real
  compiler command.

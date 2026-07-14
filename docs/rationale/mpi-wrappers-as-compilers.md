# MPI wrappers are recorded as compilers, not expanded

## Context

MPI wrapper commands (`mpicc`, `mpicxx`, `mpifort`, ... from Open MPI
and MPICH; `mpiicc`, `mpiifort`, ... from Intel MPI) wrap the real
compiler, but they are a different kind of wrapper than ccache/distcc/
sccache: the real compiler is not an argument of the invocation. The
underlying compiler and the extra `-I`/`-L` flags are baked into the
wrapper at MPI-installation time and are discoverable only by running
the wrapper itself (`mpicc -show` on MPICH, `mpicc -showme` on Open
MPI). Bear's wrapper-unwrapping mechanism therefore cannot apply.

Two options were on the table:

- **(a) Treat the wrapper itself as the compiler** and record the
  invocation verbatim, parsing its flags with the flag table of the
  compiler family it fronts.
- **(b) Expand at analysis time** by running `mpicc -show` (or
  `-showme`) and recording the underlying compiler command with the
  baked-in flags inlined.

Option (b) executes tools during semantic analysis, which is slow (one
fork+exec per invocation or at least per wrapper path), couples Bear to
the presence and health of the MPI installation at analysis time, and
differs per MPI vendor (`-show` vs `-showme` vs `-compile_info`). The
expanded command is also not necessarily what clang tooling needs:
clangd already solves exactly this problem on its side with
`--query-driver`, which asks the wrapper for its implicit include paths.

## Decision

Option (a): record the wrapper invocation verbatim. Flag parsing uses
the GCC flag table (via table inheritance), because the wrappers pass
their arguments through to a gcc-compatible driver. Intel MPI wrappers
are the exception: they are recognized as the Intel compilers
themselves (extra basenames on the existing Intel families), because
`mpiicc` is `icc`/`icx` with MPI paths baked in and Intel's flag table
has arity entries (e.g. `-debug` takes an argument) that the GCC table
would mis-parse.

## Consequences

- No process execution during analysis; recognition stays pure and
  works even when the MPI toolchain is not installed on the machine
  that runs `bear semantic`.
- Clang tooling users may need `--query-driver` (clangd) or an
  equivalent mechanism to learn the wrapper's baked-in include paths.
  Revisit expansion only if users ask for it.
- Wrapper-info invocations (`-showme`, `-show`, `-compile_info`,
  `-link_info`) classify as info-and-exit, so probing the wrapper does
  not pollute the database.
- In preload mode the wrapper's child compiler exec is intercepted too;
  the default duplicate filter (directory+file) collapses the pair and
  the wrapper entry wins because its event comes first.

## References

- Requirement: `recognition-compiler-names`
- Open MPI wrapper docs: <https://docs.open-mpi.org/en/main/building-apps/customizing-wrappers.html>
- MPICH wrapper docs: `mpicc(1)`, `-show`/`-compile_info`/`-link_info`

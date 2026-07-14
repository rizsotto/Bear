---
title: Recognize AMD ROCm compiler names
status: implemented
---

## Intent

Builds using AMD's ROCm toolchain record their compilations in the
compilation database without any configuration. The recorded compiler is
the name the build invoked, exactly as executed.

## Acceptance criteria

- Executions of `amdclang` and `amdclang++` with a source file yield
  database entries, parsed with Clang flag semantics.
- Executions of `amdflang` with a source file yield a database entry,
  parsed with Flang flag semantics.
- Executions of `hipcc` with a source file yield a database entry,
  parsed with Clang flag semantics.
- AOCC's plain `clang`/`clang++`/`flang` names are unaffected; they were
  already recognized before this requirement.
- Tool names that merely share the `amd`/`hip` prefix but are not
  compiler drivers (for example `amdgpu-arch`, which reports target GPU
  architectures) are not recognized as compilers.

## Testing

Given an event file with a `hipcc -c hello.c` execution:

> When `bear semantic` runs,
> then the database contains one entry for `hello.c`
> whose arguments start with `hipcc`.

Given the executable names `amdclang`, `amdclang++`, `hipcc`, and
`amdflang`:

> When Bear recognizes each of them,
> then `amdclang` and `amdclang++` dispatch to the Clang interpreter,
> `hipcc` dispatches to the Clang interpreter,
> and `amdflang` dispatches to the Flang interpreter.

Given the executable name `amdgpu-arch`:

> When Bear attempts to recognize it,
> then it is not recognized as a compiler.

## Notes

- `hipcc` is included even though the official ROCm documentation
  (https://rocm.docs.amd.com/projects/HIPCC/en/latest/) describes it as
  "a compiler driver utility that will call clang or nvcc, depending on
  target, and pass the appropriate include and library options". Being a
  driver does not disqualify it: gcc and clang are themselves drivers
  over an internal compilation stage that Bear already recognizes and
  ignores, and the CUDA compiler driver `nvcc` -- the same kind of driver
  over a host compiler -- is already recognized as a compiler. Bear
  records the user-facing driver invocation, the same way it does for
  `nvcc`; `hipcc` accepts clang-style options, so it dispatches with
  Clang flag semantics.
- In preload mode the child compiler that `hipcc`, `amdclang`, or
  `amdflang` execs into is intercepted too, so the default duplicate
  filter collapses the driver and child events for one source into a
  single entry, keeping the driver invocation (see
  `output-duplicate-detection`).
- AOCC (AMD's Clang-based CPU compiler) installs plain `clang`/`clang++`/
  `flang` binaries, not AMD-prefixed names; those were already covered by
  the existing `clang`/`flang` recognition before this requirement.
- No new compiler family is introduced: `amdclang`, `amdclang++`, and
  `hipcc` are additional recognized names for the existing Clang
  interpreter; `amdflang` is an additional recognized name for the
  existing Flang interpreter.

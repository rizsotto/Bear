# Wrapper mechanism rewrites program variables only

## Context

The wrapper mechanism points environment variables at the wrapper
executables. Two kinds of variable could be in scope: variables that name
a program (`CC`, `CXX`, `CUDACXX`), and variables that carry flags
(`CFLAGS`, `LDFLAGS`, `RUSTFLAGS`).

Flag variables are not read by compilers. Make expands them into the
command line, so Bear sees the flags in the arguments already. Capturing
them only fills the recorded environment with data nobody uses.

The cargo variables (`CARGO`, `RUSTC`, `RUSTC_WRAPPER`) made the wrapper
mechanism wrap cargo and rustc. Bear does not record rustc calls, so the
wrapping gave no entry and only added a hop to every Rust build step.

The program variable list follows make's implicit variables and CMake's
language variables. CMake reads them on first configure to pick the
compiler, so a build that sets them needs the wrapper there.

## Decision

Rewrite make's program variables and CMake's language program variables.
Do not rewrite or capture flag, architecture, or target variables, and do
not rewrite cargo variables.

## Consequences

Cargo builds run untouched. A new build system with a program variable
means one more line in the list. The recorded environment is advisory
only (see
[interception-events-format](../requirements/interception-events-format.md)),
so dropping flag variables from it loses no database content.

## References

- [interception-wrapper-mechanism](../requirements/interception-wrapper-mechanism.md), the requirement this supports
- [interception-events-format](../requirements/interception-events-format.md), the recorded environment is advisory
- [CMake environment variables for languages](https://cmake.org/cmake/help/latest/manual/cmake-env-variables.7.html#environment-variables-for-languages)

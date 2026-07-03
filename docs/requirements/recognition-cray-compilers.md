---
title: Recognize Cray CCE C/C++ compilers
status: implemented
---

## Intent

Builds on HPE Cray systems using the Cray Compiling Environment (CCE)
record their C/C++ compilations in the database without any
configuration, matching the existing support for the Cray Fortran
compiler.

## Acceptance criteria

- Executions of the CCE C/C++ compiler names, including versioned
  variants, with a source file yield database entries, parsed with
  Clang flag semantics (CCE C/C++ is Clang-based).
- The PrgEnv wrapper `CC` is classified by probing the executable
  (the same contract as `cc`/`c++` in the ambiguous-name-probe
  requirement), because the basename alone is ambiguous: on HPE Cray
  systems `CC` drives whichever compiler module is currently loaded,
  which may be CCE, GCC, or another vendor's compiler depending on the
  loaded programming environment.
- The `cc` and `ftn` PrgEnv wrappers keep working exactly as today
  (version probe and Cray Fortran recognition respectively); this
  requirement does not change their behavior.

## Testing

Given an event file with a CCE C/C++ compiler execution compiling a
source file:

> When `bear semantic` runs,
> then the database contains one entry for that source file.

Given a host where the PrgEnv wrapper `CC` resolves to the CCE Clang
frontend:

> When Bear recognizes an execution of `CC -c hello.c`,
> then it dispatches to the Clang interpreter.

Given a host where the PrgEnv wrapper `CC` resolves to GCC (PrgEnv-gnu):

> When Bear recognizes an execution of `CC -c hello.c`,
> then it dispatches to the GCC interpreter.

Given an execution of the `cc` or `ftn` PrgEnv wrapper:

> When Bear recognizes it,
> then behavior is unchanged from before this requirement (probe for
> `cc`, Cray Fortran recognition for `ftn`).

## Notes

- `CC` is added to the same ambiguous-name probe set as `cc`/`c++`; it
  is deliberately absent from every recognition pattern, for the same
  reason `cc`/`c++` are absent (see
  `recognition-ambiguous-name-probe.md`). A static mapping would be
  wrong on every programming environment except the one it hardcodes,
  and on case-insensitive filesystems it would shadow `cc`.
- Known limitation: the probe classifies only the version banners it
  already recognizes (Clang's and GCC's). A programming environment
  whose compiler prints a banner the probe does not know -- for
  example `nvc++` under PrgEnv-nvidia -- stays unrecognized. Extending
  the classifier to more banners is out of scope for this requirement.

## Rationale

- [Cached version probe as sole classifier](../rationale/ambiguous-cc-version-probe.md) -
  the same decision that governs `cc`/`c++` applies to `CC` without
  changes.

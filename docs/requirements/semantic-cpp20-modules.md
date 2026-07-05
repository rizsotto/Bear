---
title: Recognize C++20 module compilations
status: implemented
---

## Intent

When a user builds a project that uses C++20 modules, the build invokes the
compiler on module-interface source files (typically `.cppm`, `.ixx`, `.ccm`)
and on module-related driver flags (e.g. `clang++ --precompile`,
`-fmodule-file=`). The user expects the generated `compile_commands.json`
to include these compilations the same way it includes ordinary `.cpp`
translation units. Clang tooling (clangd, clang-tidy) needs the module
inputs and module-path flags to follow imports and produce accurate
diagnostics.

Today these entries are dropped: the module-interface extensions are not
in the source-extension table, so the heuristic finds no source and
filters the invocation out. The module-related driver flags do not by
themselves break recognition -- an unrecognized single-token flag is
consumed individually and never fabricates or destroys arity -- but the
per-compiler flag tables must still classify them so pass detection stays
accurate (e.g. `--precompile` is a compile action, not a link).

## Acceptance criteria

- The source-extension table recognizes the standard C++20 module-interface
  extensions: `.cppm`, `.ixx`, `.mxx`, `.ccm`, `.cxxm`, `.c++m`.
- `clang.yaml` and `gcc.yaml` classify the following flags so they do not
  break recognition or argument-arity tracking:
  - `--precompile` (Clang module-interface compile)
  - `-fmodule-file=*` (Clang/GCC, including the `<name>=<file>` form)
  - `-fmodule-map-file=*`
  - `-fmodule-name=*`
  - `-fmodules`, `-fno-modules`, `-fcxx-modules`
  - `-fmodules-ts` (Clang transitional)
  - `-fprebuilt-module-path=*`
  - `-fbuiltin-module-map`
  - `-fimplicit-modules`, `-fno-implicit-modules`
  - GCC: `-fmodules-ts`, `-fmodule-header`, `-fmodule-only`,
    `-fmodule-mapper=*`, `-fmodule-lazy`
- A `clang++ --precompile -std=c++20 foo.cppm -o foo.pcm` invocation
  produces one entry whose `file` is `foo.cppm` and whose arguments are
  preserved verbatim.
- A `clang++ -std=c++20 -fmodule-file=foo=foo.pcm -c main.cpp` invocation
  produces one entry whose `file` is `main.cpp` with the
  `-fmodule-file=foo=foo.pcm` flag preserved.
- Precompiled module artifacts (`.pcm`, BMI files) are not classified as
  sources. They appear on argv but never as the entry's `file` field.

## Non-functional constraints

- The recognition layer must remain extension-driven for the source check.
  Probe-based detection of module-interface units is out of scope.
- Adding flag entries must not perturb the codegen snapshot tests in
  `compilers-codegen`. Regeneration via `cargo build` is expected.

## Testing

Given a build that compiles a module-interface unit:

> When the user runs `bear -- clang++ -std=c++20 --precompile foo.cppm -o foo.pcm`,
> then the resulting `compile_commands.json` contains one entry whose
> `file` is `foo.cppm` and whose `arguments` preserve `--precompile`
> and the input/output paths.

Given a build that consumes a precompiled module:

> When the user runs `bear -- clang++ -std=c++20 -fmodule-file=foo=foo.pcm -c main.cpp`,
> then the resulting `compile_commands.json` contains one entry whose
> `file` is `main.cpp` with the `-fmodule-file=foo=foo.pcm` flag preserved,
> and no entry is produced for `foo.pcm`.

Given a mixed build with one module interface and one consumer:

> When the user runs both commands above in sequence,
> then `compile_commands.json` contains exactly two entries
> (one for `foo.cppm`, one for `main.cpp`).

Given a GCC build using `-fmodules-ts`:

> When the user runs `bear -- g++ -std=c++20 -fmodules-ts -c mod.cppm`,
> then the resulting `compile_commands.json` contains one entry whose
> `file` is `mod.cppm`.

## Notes

- GitHub issue #637 reported that `.cppm` compilations are silently
  dropped from the database. The issue was bulk-closed in 2026 presuming
  the rewrite addressed it; it did not.
- Source-extension list lives at
  `crates/bear/src/semantic/interpreters/matchers/source.rs`.
- Flag tables live in `crates/bear/compilers/clang.yaml` and
  `crates/bear/compilers/gcc.yaml`. `clang.yaml` already `extends: gcc`, so
  module flags common to both should land in `gcc.yaml` and Clang-only
  flags in `clang.yaml`.
- Out of scope: module dependency-graph awareness, BMI cache discovery,
  and any module-map authoring. This requirement is strictly about
  capturing the invocations.

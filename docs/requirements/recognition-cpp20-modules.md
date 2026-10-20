---
title: Recognize C++20 module compilations
status: implemented
---

## Intent

When a project uses C++20 modules, the build compiles module-interface
source files (typically `.cppm`, `.ixx`, `.ccm`) and passes
module-related flags (`clang++ --precompile`, `-fmodule-file=`). The
user expects `compile_commands.json` to hold these compilations the same
way it holds ordinary `.cpp` translation units. Clang tooling (clangd,
clang-tidy) needs the module inputs and module flags to follow imports
and give accurate diagnostics.

## Acceptance criteria

- A source file with a module-interface extension is a translation unit
  like any other: `.cppm`, `.ixx`, `.mxx`, `.ccm`, `.cxxm`, `.c++m`.
- Module-related compiler flags are recognized. With or without a
  value, they never take a following source file as their own and never
  suppress or corrupt the entry. The recognized set is the one Bear's
  compiler definitions carry.
- A precompiled module (`.pcm`, a BMI file) on the command line is never
  the entry's `file`.
- **Given** a module-interface compile, **when** the user runs
  `bear -- clang++ -std=c++20 --precompile foo.cppm -o foo.pcm`,
  **then** the output holds one entry with `file` `foo.cppm`, and its
  `arguments` keep `--precompile` and the input and output paths.
- **Given** a build that uses a precompiled module, **when** the user
  runs `bear -- clang++ -std=c++20 -fmodule-file=foo=foo.pcm -c main.cpp`,
  **then** the output holds one entry with `file` `main.cpp` that keeps
  `-fmodule-file=foo=foo.pcm`, and no entry for `foo.pcm`.
- **Given** a build that runs both commands above, **when** Bear
  finishes, **then** the output holds exactly two entries, one for
  `foo.cppm` and one for `main.cpp`.
- **Given** a GCC module build, **when** the user runs
  `bear -- g++ -std=c++20 -fmodules-ts -c mod.cppm`, **then** the output
  holds one entry with `file` `mod.cppm`.

## Non-functional constraints

- A module-interface source is detected by its file extension only.
  Detecting it by probing the compiler is out of scope.

## Known limitations

Bear captures the module compile commands and nothing more. It does not
track the module dependency graph, find BMI caches, or write module maps.

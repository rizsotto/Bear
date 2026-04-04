---
id: output-001
title: JSON compilation database output
status: implemented
tests:
  - test_basic_c_compilation
  - test_basic_cxx_compilation
---

## Intent

When the user runs Bear wrapping a build command, Bear produces a JSON file
(`compile_commands.json` by default) that lists every compilation command
invoked during the build. Each entry contains the working directory, the
source file, and the compilation command or arguments.

## Acceptance criteria

- Output file is valid JSON
- Each entry contains `directory`, `file`, and at least one of `command` or `arguments`
- Entries correspond to actual compiler invocations observed during the build
- Non-compiler commands (linker-only, preprocessor-only queries) are excluded
- Output path is configurable via `--output` flag

## Non-functional constraints

- Output must conform to the Clang JSON Compilation Database specification
- Must work on Linux, macOS, and Windows

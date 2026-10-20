---
title: JSON compilation database output
status: implemented
---

## Intent

When the user runs Bear on a build, Bear writes a JSON compilation
database (`compile_commands.json` by default) that lists the compiler
calls of the build. Each entry holds the working directory, the source
file, and the compile command. Which calls produce entries, and in what
shape, is owned by `output-compilation-entries`. One malformed entry
never costs the user the rest of the database.

Related contracts have their own owners: how many entries one file may
keep is owned by `output-duplicate-detection`, and how the `file`,
`directory`, and `output` paths are spelled is owned by
`output-path-format`.

## Acceptance criteria

- **Given** a build that compiles one C source file, **when** Bear
  finishes, **then** the output is valid JSON with exactly one entry,
  whose `directory` is the working directory, whose `file` is the
  source, and whose `arguments` start with the compiler.
- **Given** a build script that compiles several C and C++ sources,
  **when** Bear finishes, **then** the output holds one entry per
  source, each with the right compiler (C or C++) in `arguments[0]`.
- **Given** a build that runs no compiler, **when** Bear finishes,
  **then** the output is an empty JSON array `[]`.
- **Given** a build where some sources compile and some fail, **when**
  Bear finishes, **then** the output holds an entry for every attempted
  compilation. The exit code is owned by `cli-exit-codes`.
- **Given** the user sets the output path to `build/commands.json`,
  **when** Bear finishes, **then** that file holds the entries, and no
  `compile_commands.json` is written in the working directory.
- **Given** the default configuration, **when** Bear writes an entry,
  **then** the entry carries the `arguments` array and no `command`
  field.
- **Given** the command-string format is selected and a compiler call
  with the argument `-DNAME="hello"`, **when** Bear writes the entry,
  **then** the entry carries `command` and no `arguments`. The value is
  the argument list joined with POSIX shell quoting (single- or
  double-quoted, both are valid) and then JSON-encoded, so in the file
  that argument reads `'-DNAME=\"hello\"'` or
  `\"-DNAME=\\\"hello\\\"\"`. JSON-decoding and then shell-word
  splitting gives back the original argument list.
- **Given** an execution whose compiler is spelled `gcc` (a bare name)
  or `/usr/bin/gcc` (a path), **when** Bear writes the entry, **then**
  `arguments[0]` is that spelling, unchanged. The output stage never
  resolves, absolutizes, or rewrites the compiler path. What spelling
  each source observes is owned elsewhere. For the preload mechanism,
  `interception-events-format` says the recorded path is absolute or a
  bare name as the caller spelled it. The others are owned by
  `interception-wrapper-mechanism` and
  `interception-shell-text-parsing`.
- **Given** an entry that fails validation, **when** Bear writes the
  output, **then** the entry is dropped, a `WARN` log line names its
  `file`, its `directory`, and the reason, the run summary counts it,
  and the entries after it are still written. An entry fails validation
  when:
  - its `file` or `directory` is empty;
  - it has both `command` and `arguments`, or neither;
  - its `command` cannot be split as POSIX shell words (for example the
    unterminated quote in `cc -c "src.c`).
- **Given** a build whose entries are all valid but one with an empty
  `directory`, **when** Bear writes the output, **then** the valid
  entries appear unchanged, and the drop does not change the exit code
  (owned by `cli-exit-codes`).
- **Given** a build where every candidate entry fails validation,
  **when** Bear writes the output, **then** the database is written as
  an empty JSON array `[]`, each drop has its `WARN` line, and a single
  `ERROR` line reports that every entry was dropped.

## Non-functional constraints

- The output conforms to the
  [Clang JSON Compilation Database specification](https://clang.llvm.org/docs/JSONCompilationDatabase.html).
- Works on Linux, macOS, and Windows.

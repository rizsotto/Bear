---
title: JSON compilation database output
status: implemented
---

## Intent

When the user runs Bear wrapping a build command, Bear produces a JSON file
(`compile_commands.json` by default) that lists every compilation command
invoked during the build. Each entry contains the working directory, the
source file, and the compilation command or arguments.

## Format specification

Bear's output conforms to the Clang JSON Compilation Database
specification, which defines the array-of-command-objects structure and
the `directory`, `file`, `arguments`, `command`, and `output` fields:
<https://clang.llvm.org/docs/JSONCompilationDatabase.html>

Two aspects of that format are specific to how Bear writes it and are
described below: how Bear escapes the `command` field, and how it spells
the compiler path in `arguments[0]`. Whether an entry carries the
optional `output` field, and its value, is owned by
`output-compilation-entries`.

### The `command` field in detail

When Bear emits the `command` field instead of `arguments`, it joins the
argument list into a single string using POSIX shell quoting conventions,
which may produce either single-quoted or double-quoted output depending
on the argument content. Both forms are valid per the specification.

The content therefore has two layers of escaping:

1. **Shell escaping** -- arguments containing spaces, quotes, or backslashes
   are quoted.
2. **JSON escaping** -- the shell-escaped string is then JSON-encoded, so
   `"` becomes `\"` and `\` becomes `\\` at the JSON level.

For example, compiling with `-DNAME=\"hello\"`:
- `arguments` form: `[..., "-DNAME=\"hello\"", ...]` (no shell escaping,
  only JSON encoding of the raw argument)
- `command` form: `"... '-DNAME=\"hello\"' ..."` or
  `"... \"-DNAME=\\\"hello\\\"\" ..."` (shell-quoted, then JSON-encoded)

### The compiler path (`arguments[0]`)

The specification states that `arguments[0]` should be the executable name
(e.g. `clang++`), but does not prescribe whether it must be an absolute
path, a relative path, or a bare command name. Bear writes the compiler
exactly as the interception layer observed it; the output stage never
resolves, absolutizes, or otherwise rewrites the path.

What "observed" means depends on the interception mode:

- Preload mode observes the literal `exec` call: if the build invoked
  `gcc`, Bear writes `gcc`; if it invoked `/usr/bin/gcc`, Bear writes
  `/usr/bin/gcc`.
- Wrapper mode cannot observe the name the build used -- by construction
  the build executed the wrapper -- so the report names the real
  compiler by its absolute path (see `interception-wrapper-mechanism.md`),
  and the database shows that absolute path.
- Shell-text parsing observes the command line as the build system wrote
  it, so the database shows exactly that spelling: a bare `gcc` in the
  text stays `gcc` (see `interception-events-from-shell-text`). No
  command ran, so no resolved path exists to report.

## Acceptance criteria

- Output file is valid JSON
- Each entry contains `directory`, `file`, and at least one of `command` or `arguments`
- The `command` and `arguments` fields are mutually exclusive in each entry
- A `command` field that cannot be parsed by POSIX shell-word splitting is
  rejected during validation
- Empty `file` or `directory` fields are rejected during validation
- An entry that fails validation is dropped from the output, logged at
  `WARN` level with the reason, and counted in the run summary; it
  never aborts processing of subsequent entries
- When every entry that would otherwise have been written is dropped due to
  validation failures, Bear emits a single `ERROR`-level summary line so
  the empty compilation database is never silent
- Entries correspond to actual compiler invocations observed during the
  build; which invocations produce entries is owned by
  `output-compilation-entries`
- The output path is configurable
- Default output format uses `arguments` (array form)
- When `command` format is selected, arguments are shell-escaped
  following POSIX shell quoting conventions

## Validation failure handling

When an entry fails validation:

- The entry is dropped and does not appear in `compile_commands.json`.
- A `WARN`-level log line names the `file`, `directory`, and the specific
  validation reason (e.g. empty `directory`, unparsable `command`).
- The run summary counts the dropped entry.
- Processing of subsequent entries continues unaffected.

This contract ensures a single malformed entry cannot destroy the usable
output produced from the rest of a build.

When every entry that would otherwise have been written is dropped, Bear
emits a single `ERROR`-level summary line stating so. The compilation
database is still written (as an empty array) so downstream tooling sees a
valid file, but the log makes the empty result impossible to miss.

Validation drops never affect the exit code; the exit-code contract
across modes is owned by [`cli-exit-codes`](cli-exit-codes.md).

## Non-functional constraints

- Output must conform to the Clang JSON Compilation Database specification
- Must work on Linux, macOS, and Windows

## Testing

Given a project with a single C source file:

> When the user runs `bear -- <compiler> -c source.c`
> then `compile_commands.json` is created,
> it contains valid JSON with exactly one entry,
> the entry has `directory` equal to the working directory,
> `file` equal to "source.c",
> and `arguments` starting with the compiler path.

Given a project with multiple C and C++ source files:

> When the user runs `bear -- sh build.sh` where build.sh compiles all files,
> then `compile_commands.json` contains one entry per source file,
> and each entry has the correct compiler (C or C++) in `arguments[0]`.
> Note: exact entry count may vary when a caching compiler wrapper
> (ccache) is in the path.

Given a build command that produces no compiler invocations:

> When the user runs `bear -- true`,
> then `compile_commands.json` contains an empty JSON array `[]`.

Given a build that partially fails (some files compile, some do not):

> When the user runs `bear -- sh build.sh`,
> then `compile_commands.json` still contains entries for all attempted
> compilations, and Bear's exit code reflects the build failure.

Given a compiler invocation with `-DNAME=\"hello\"`:

> When Bear writes the `command` field,
> the value is shell-escaped (the crate may use single or double quotes),
> the JSON encoding adds another layer,
> and JSON-decoding followed by shell-word splitting recovers the original argv.

Given a compiler invoked as a bare name (e.g. `gcc`), intercepted in
preload mode:

> When Bear writes the entry,
> then `arguments[0]` is `gcc` (not resolved to an absolute path).

Given a compiler invoked with a full path (e.g. `/usr/bin/gcc`):

> When Bear writes the entry,
> then `arguments[0]` is `/usr/bin/gcc`.

Given a build that produces a mix of valid entries and one entry that
fails validation (for example, an empty `directory` field):

> When Bear writes the output,
> then the valid entries appear in `compile_commands.json` unchanged,
> a `WARN` log line names the dropped entry and the validation reason,
> the run summary reports one dropped entry,
> and the process exit code is not affected by the drop.

Given a build where every candidate entry fails validation:

> When Bear writes the output,
> then `compile_commands.json` is written as an empty JSON array `[]`,
> a `WARN` log line is emitted for each dropped entry,
> and a single `ERROR`-level summary line reports that every entry was
> dropped.

## Notes

- The specification allows multiple entries for the same file (different
  configurations). Bear does not merge or deduplicate across configurations
  unless the duplicate filter removes them (see `output-duplicate-detection`).
- Path formatting for the `file` and `directory` fields is configurable;
  see `output-path-format` for details.
- The `arguments[0]` spelling contract responds to issues #240, #678,
  and #671.
- The validation-drop contract responds to issue #692.

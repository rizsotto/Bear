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

Bear's output conforms to the Clang JSON Compilation Database specification:
<https://clang.llvm.org/docs/JSONCompilationDatabase.html>

A compilation database is a JSON file consisting of an array of "command
objects", where each command object specifies one way a translation unit is
compiled in the project.

### Example

```json
[
  { "directory": "/home/user/llvm/build",
    "arguments": ["/usr/bin/clang++", "-Irelative",
      "-DSOMEDEF=With spaces, quotes and \\-es.",
      "-c", "-o", "file.o", "file.cc"],
    "file": "file.cc" },

  { "directory": "/home/user/llvm/build",
    "command": "/usr/bin/clang++ -Irelative \"-DSOMEDEF=With spaces, quotes and \\-es.\" -c -o file.o file.cc",
    "file": "file2.cc" }
]
```

### Field definitions

**`directory`** -- The working directory of the compilation. All paths
specified in the `command` or `file` fields must be either absolute or
relative to this directory.

**`file`** -- The main translation unit source processed by this compilation
step. This is used by tools as the key into the compilation database. There
can be multiple command objects for the same file, for example if the same
source file is compiled with different configurations.

**`arguments`** -- The compile command argv as a list of strings. This should
run the compilation step for the translation unit `file`. `arguments[0]`
should be the executable name, such as `clang++`. Arguments should not be
escaped, but ready to pass to `execvp()`.

**`command`** -- The compile command as a single shell-escaped string.
Arguments may be shell quoted and escaped following platform conventions,
with `"` and `\` being the only special characters. Shell expansion is not
supported.

Either `arguments` or `command` is required. `arguments` is preferred, as
shell (un)escaping is a possible source of errors.

**`output`** -- The name of the output created by this compilation step. This
field is optional. It can be used to distinguish different processing modes
of the same input file.

### The `command` field in detail

When Bear emits the `command` field (instead of `arguments`), it joins the
argument list into a single string using `shell_words::join`. The resulting
string is then embedded in JSON.

The `shell_words` crate follows POSIX shell quoting conventions and may
produce either single-quoted or double-quoted output depending on the
argument content. Both forms are valid per the specification.

This means the content has two layers of escaping:

1. **Shell escaping** -- arguments containing spaces, quotes, or backslashes
   are quoted. The crate chooses single or double quotes as appropriate.
2. **JSON escaping** -- the shell-escaped string is then JSON-encoded, so
   `"` becomes `\"` and `\` becomes `\\` at the JSON level.

For example, compiling with `-DNAME=\"hello\"`:
- `arguments` form: `[..., "-DNAME=\"hello\"", ...]` (no shell escaping,
  only JSON encoding of the raw argument)
- `command` form: `"... '-DNAME=\"hello\"' ..."` or
  `"... \"-DNAME=\\\"hello\\\"\" ..."` (shell-quoted, then JSON-encoded)

Consumers that read the `command` field must first JSON-decode the string,
then apply shell unquoting to recover the original argv. This double
encoding has historically been a source of bugs (see GitHub issues #14, #70,
#77, #81, #88, #96, #508).

### The compiler path (`arguments[0]`)

The specification states that `arguments[0]` should be the executable name
(e.g. `clang++`), but does not prescribe whether it must be an absolute
path, a relative path, or a bare command name. Bear preserves the compiler
path as it was observed during interception -- if the build invoked `gcc`,
Bear writes `gcc`; if it invoked `/usr/bin/gcc`, Bear writes `/usr/bin/gcc`.

This behavior differs from Bear v3.x, which resolved compiler paths to
absolute. The current behavior is configurable but the specification is
intentionally silent on this point.

Related issues: #240, #678, #679, #671.

## Acceptance criteria

- Output file is valid JSON
- Each entry contains `directory`, `file`, and at least one of `command` or `arguments`
- The `command` and `arguments` fields are mutually exclusive in each entry
- A `command` field that cannot be parsed by POSIX shell-word splitting is
  rejected during validation
- Empty `file` or `directory` fields are rejected during validation
- Entries correspond to actual compiler invocations observed during the build
- Non-compiler commands (linker-only, preprocessor-only, info-only such as
  `--version` or `--help`) are excluded
- Output path is configurable via `--output` flag
- Default output format uses `arguments` (array form)
- When `command` format is selected, arguments are shell-escaped using
  `shell_words::join`
- The `output` field is omitted by default and included when
  `format.entries.include_output_field` is enabled

## Implementation details

Bear defaults to the `arguments` array format because the specification
recommends it and because shell (un)escaping is a known source of errors.
The `command` string format is available for consumers that require it.

The format selection is controlled via configuration:

```yaml
format:
  entries:
    use_array_format: true        # true = arguments, false = command
    include_output_field: false   # include the output field
```

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

Given a compiler invoked as a bare name (e.g. `gcc`):

> When Bear writes the entry,
> then `arguments[0]` is `gcc` (not resolved to an absolute path).

Given a compiler invoked with a full path (e.g. `/usr/bin/gcc`):

> When Bear writes the entry,
> then `arguments[0]` is `/usr/bin/gcc`.

## Notes

- The specification allows multiple entries for the same file (different
  configurations). Bear does not merge or deduplicate across configurations
  unless the duplicate filter removes them (see `output-duplicate-detection`).
- Path formatting for the `file` and `directory` fields is configurable;
  see `output-path-format` for details.

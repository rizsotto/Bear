---
title: Path format for file and directory fields
status: implemented
---

## Intent

The [JSON compilation database specification](https://clang.llvm.org/docs/JSONCompilationDatabase.html)
allows paths that are absolute or relative to the entry's directory.
Tools that read the database want different forms. Some need absolute
paths, some work better with relative ones, and some need symlinks
resolved. Bear lets the user choose the path format of the `directory`
field and of the `file` field.

## Acceptance criteria

Four formats are supported for `file`, and all but the relative one for
`directory`:

| Format | Result |
|---|---|
| as is (the default) | the path exactly as interception observed it |
| absolute | the path made absolute; it need not exist on disk |
| relative (`file` only) | the path relative to the entry's directory |
| symlink-resolving | symlinks followed and `.` and `..` removed; the path must exist when the output is written |

- **Given** a build run from `/home/user/project` that compiles
  `src/main.c`, **when** both fields use the default format, **then**
  `directory` and `file` are written as interception observed them,
  with `file` as `src/main.c`.
- **Given** an event that records the working directory as the relative
  path `build`, while Bear runs from `/home/user/project`, **when**
  `directory` uses the absolute format, **then** `directory` is
  `/home/user/project/build`. The `directory` field is the working
  directory of the compiler call, formatted with itself as the base.
- **Given** a source given as `/home/user/project/src/main.c` and a
  directory of `/home/user/project`, **when** `file` uses the relative
  format, **then** `file` is `src/main.c`. The `file` field is resolved
  against the already formatted `directory`.
- **Given** `/a/b/one.c` compiled from `/a/b` and `/a/c/two.c` compiled
  from `/a/c`, **when** `directory` is absolute and `file` is relative,
  **then** the entries are `/a/b` with `one.c` and `/a/c` with `two.c`.
- **Given** a configuration that selects the relative format for
  `directory`, **when** Bear starts, **then** it exits non-zero before
  any work starts, and the diagnostic names the setting and the
  absolute format as the alternative. Released clang tools do not use
  an entry whose `directory` and `file` are both relative.
- **Given** a source reached through a symlink, **when** `file` uses the
  symlink-resolving format, **then** `file` is the real path.
- **Given** a build on Windows, **when** a field uses the
  symlink-resolving format, **then** the path has no `\\?\`
  extended-length prefix (issue #683).
- **Given** a source that no longer exists when the output is written,
  **when** `file` uses the symlink-resolving format, **then** Bear logs a
  warning and writes the path as interception observed it.
- **Given** a working directory that no longer exists when the output is
  written, **when** `directory` uses the symlink-resolving format,
  **then** the entry is dropped. The drop follows the same contract as a
  validation drop: it is named in a warning, counted in the run summary,
  and part of the error when every entry is dropped (owned by
  `output-json-compilation-database`).
- **Given** a build run from `/home/user/project` that compiles
  `src/main.c` to `obj/main.o`, with the `output` field on, **when**
  `file` uses the relative format, **then** `output` is `obj/main.o`.
  The `output` field always uses the format of `file`.
- **Given** the same build where `obj/main.o` was deleted before the
  output is written, **when** `file` uses the symlink-resolving format,
  **then** `file` is the real path, and `output` falls back to
  `obj/main.o` with a warning.
- **Given** a build run from `/home/user/project` that compiles
  `src/main.c` to `obj/main.o` with `-I../include`, **when** `file` uses
  the absolute format, **then** `arguments` carry the source as
  `/home/user/project/src/main.c` and the output as
  `/home/user/project/obj/main.o`, and `-I../include` is unchanged. The
  source and output paths in `arguments` always use the format of
  `file`.
- The symlink-resolving format is rejected for input declared to come
  from another machine (`cli-input-from-another-machine`).

## Non-functional constraints

- The relative format handles paths that leave the directory, such as
  `../../other/dir/file.c`.

## Known limitations

- Paths inside other flags, such as include search paths, are not
  transformed. They are written as the build spelled them.

## Rationale

- [No relative format for `directory`](../rationale/no-relative-directory-format.md)

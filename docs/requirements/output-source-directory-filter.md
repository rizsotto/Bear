---
title: Source directory filtering
status: implemented
---

## Intent

Users often want to keep some sources out of the database. Files under
`/usr/include`, generated files in `build/`, or test-only sources in
`src/test/` clutter it and may confuse tools like clangd or clang-tidy.
Directory rules let the user include or exclude entries by the path of
their source file.

## Acceptance criteria

- **Given** no directory rules, **when** Bear writes the database,
  **then** every entry is kept, whatever its path.
- **Given** a rule that excludes `/usr/include`, **when** a build
  compiles `src/main.c` and a file under `/usr/include`, **then** only
  `src/main.c` is written.
- **Given** the rules include `src`, exclude `src/test`, include
  `src/test/integration`, in that order, **when** a build compiles
  files in all three, **then** `src/main.c` is kept, `src/test/unit.c`
  is dropped, and `src/test/integration/api.c` is kept. Rules are
  evaluated in order, and the last matching rule wins.
- **Given** rules that cover only `src`, **when** a build compiles
  `lib/external.c`, **then** it is kept. A file no rule matches is kept.
- **Given** a rule for `src`, **when** a build compiles `src/main.c` and
  `not_src/main.c`, **then** only `src/main.c` matches. A rule matches
  whole path components, never part of a name. It matches files in
  that directory and in every subdirectory.
- **Given** a rule that excludes the file `src/main.c`, **when** a build
  compiles `src/main.c` and `src/main.cpp`, **then** `src/main.c` is
  dropped and `src/main.cpp` is kept.
- **Given** a rule that excludes the absolute path `/usr/include`,
  **when** a build compiles `src/main.c`, recorded with that relative
  path, **then** `src/main.c` is kept. Paths are compared as written in
  the entry, with no normalization, so rule paths must match the path
  format in use (`output-path-format`).
- **Given** a rule that excludes `src`, **when** a build compiles
  `src/main.c`, `Src/main.c`, and `SRC/main.c`, **then** only
  `src/main.c` is dropped. Matching is case-sensitive on every platform,
  Windows and macOS included.
- **Given** an entry that stands for a whole invocation as one
  translation unit (`output-compilation-entries`), **when** the rules are
  applied, **then** the entry is kept or dropped as a unit, by the one
  source in its `file` field.
- **Given** a rule with an empty path, **when** the configuration is
  validated, **then** validation fails with an error.

## Non-functional constraints

- Matching does not touch the file system: no stat calls, no symlink
  resolution.

## Known limitations

- Symlinks are not resolved. A rule for `/real/path` does not match a
  file reached through `/symlink/path`. The symlink-resolving path
  format (`output-path-format`) resolves paths before matching.
- For a one-unit invocation whose sources span an included and an
  excluded directory, only the source in `file` counts. Real builds do
  not produce such input, so the gap has no test.

## Rationale

- [Source filter: last matching rule wins](../rationale/source-filter-last-match-wins.md)

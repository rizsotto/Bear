---
title: Dropping entries whose source file is missing
status: accepted
---

## Intent

A database can hold entries for source files that no longer exist: a
build compiled a temporary file and deleted it, or an appended database
still lists a file that was removed from the project. Tools like clangd
then report errors for files the user cannot open. A configuration
option makes Bear drop every entry whose source file is missing when it
writes the database. This is on by default.

## Acceptance criteria

- **Given** a build that compiles `existing.c` and `missing.c`, where
  only `existing.c` is on disk when Bear writes the database, and the
  option is on, **when** Bear runs, **then** the database holds
  `existing.c` only.
- **Given** the same build and the option off, **when** Bear runs,
  **then** the database holds both entries.
- **Given** the same build and no setting for the option, **when** Bear
  runs, **then** the database holds `existing.c` only. On is the
  default.
- **Given** an entry whose `file` is relative, **when** the option is
  on, **then** Bear checks the file relative to the entry's `directory`,
  not relative to the directory Bear runs in.
- **Given** an existing database with an entry for `removed.c`, a file
  that is no longer on disk, and a run that is appending with the option
  on, **when** Bear finishes, **then** the database holds no entry for
  `removed.c`. Existing entries are checked the same way as new ones.
- **Given** an entry whose source file exists but cannot be checked (for
  example, a parent directory denies access), **when** the option is on,
  **then** the entry is kept. Only a file known to be missing is
  dropped.
- **Given** the option on and directory or pattern rules
  (`output-source-directory-filter`, `output-generated-file-filter`),
  **when** Bear writes the database, **then** an entry is written only
  when every one of these filters accepts it.
- **Given** an entry that stands for a whole invocation as one
  translation unit (`output-compilation-entries`), **when** the option
  is on, **then** the entry is kept or dropped as a unit, by the one
  source in its `file` field.

## Non-functional constraints

- The check runs once per entry, when the database is written. It costs
  one file-system lookup per entry and nothing when the option is off.

## Known limitations

- The check sees the file system when the entry is written, and that
  moment differs by mode. In combined mode, entries are written while
  the build runs, so a file the build deletes after compiling it
  usually passes the check and is kept. In semantic mode and parse-sh
  mode, the build is over before Bear runs, so the same file is
  dropped. Existing entries read when appending are checked after the
  build in every mode, so an appending run with a build that runs no
  compiler trims them.

## Rationale

- [Missing-source filter is on by default and checks once](../rationale/missing-source-filter-default-on.md)

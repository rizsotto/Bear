---
title: Appending to an existing compilation database
status: implemented
---

## Intent

A user who builds in steps, or builds parts of a project at different
times, wants one compilation database that collects the entries of every
Bear run. When appending, Bear merges the new entries with the existing
`compile_commands.json` instead of replacing it.

## Acceptance criteria

- **Given** an existing database with an entry for `file1.c`, **when** a
  run that is appending compiles `file2.c`, **then** the database holds
  both entries, and the new entry for `file2.c` comes before the existing
  entry for `file1.c`.
- **Given** no existing database, **when** a run that is appending
  compiles `file1.c`, **then** Bear writes a database with only that
  entry, and does not fail.
- **Given** an existing database with an entry for `file1.c`, **when** a
  run that is not appending compiles `file2.c`, **then** the database
  holds only the entry for `file2.c`. Overwriting is the default.
- **Given** the same setup, **when** the run asks for overwriting
  explicitly, **then** the result is the same. A caller can name the
  behavior it depends on instead of relying on the default.
- **Given** any run that writes a database, **when** it asks for both
  appending and overwriting, **then** Bear reports a usage error and runs
  no build.
- **Given** an existing database and a build that runs no compiler,
  **when** a run that is appending finishes, **then** the existing
  entries are kept unchanged.
- **Given** an existing database that is not valid JSON, **when** a run
  that is appending finishes, **then** Bear logs a warning for the break,
  and one more for the entry the break is inside of, if any. The database
  holds the new entries plus the existing entries read before the break.
  Entries after the break are dropped.
- **Given** an existing database with some invalid entries (complete JSON
  objects that are not valid entries), **when** a run that is appending
  finishes, **then** Bear logs one warning per invalid entry and skips
  it, and keeps the valid entries and the new ones.
- **Given** an existing database that cannot be opened (for example,
  read permission is denied), **when** a run that is appending starts,
  **then** Bear reports an error before the build runs and writes no
  output.
- Which entry survives when a file is in both the new and the existing
  entries is owned by `output-duplicate-detection`.

## Non-functional constraints

- An interrupted run does not corrupt the database. That is owned by
  `output-atomic-write`.

## Rationale

- [Latest compilation wins for a rebuilt file](../rationale/duplicate-latest-flags-win.md)
- [Naming the overwrite behaviour before it stops being the default](../rationale/append-default-migration.md)

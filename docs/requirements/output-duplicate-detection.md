---
title: Duplicate entry detection and filtering
status: implemented
---

## Intent

A build may run the compiler for the same source file more than once:
parallel make retries, ccache wrappers, or repeated builds that append to
an existing database after a file's flags change. The
[compilation database specification](https://clang.llvm.org/docs/JSONCompilationDatabase.html)
allows several entries for one file, but stale duplicates confuse tools
and keep old flags after a rebuild. By default Bear keeps one entry per
source file, identified by its `directory` and `file` fields. A user who
needs several configurations recorded for one file can widen the set of
fields that tell entries apart.

## Acceptance criteria

- **Given** a build that compiles `file.c` twice with the same flags,
  **when** Bear writes the database, **then** it holds one entry for
  `file.c`.
- **Given** a build that compiles `a.c`, `b.c`, and `c.c` in that order,
  **when** Bear writes the database, **then** the entries appear in that
  order. Kept entries stay in the order they were received.
- **Given** a build that compiles `file.c` with `-O2` and then with `-O3`,
  **when** Bear writes the database with the default match, **then** it
  holds one entry for `file.c`, the first one. The default match is the
  `directory` and `file` fields.
- **Given** a match on `directory`, `file`, and `arguments`, and the same
  build, **when** Bear writes the database, **then** it holds both
  entries.
- **Given** `src/util.c` and `lib/util.c` (same name, different
  directories), **when** Bear writes the database, **then** it holds both
  entries. Regression guard for issue #667.
- **Given** a match on `file` alone and a build that compiles `file.c`
  twice with different flags, **when** Bear writes the database, **then**
  only the first entry is kept.
- **Given** a match on `file` and `output`, and `file.c` compiled to both
  `debug/file.o` and `release/file.o`, **when** Bear writes the database,
  **then** it holds both entries.
- **Given** appending, an existing database with an entry for `file.c`,
  and a new build that compiles `file.c` with different flags, **when**
  Bear writes the database, **then** it holds one entry for `file.c`,
  with the new flags. The new entry wins because new entries come first
  (owned by `output-append`) and the first one is kept.
- **Given** a compiler driver or wrapper and the child compiler it
  spawns, both intercepted by the preload mechanism as two events for one
  source, **when** Bear writes the database with the default match,
  **then** exactly one entry survives, and it records the driver or
  wrapper invocation, which is emitted first. Regression guard for issue
  #638.
- **Given** a match configuration that is empty, lists a field twice, or
  lists both `command` and `arguments` (two forms of the same data),
  **when** Bear loads the configuration, **then** it rejects it with an
  error that names the problem.

## Non-functional constraints

- A duplicate is detected no matter how far apart the two occurrences
  are in the input.

## Rationale

- [Latest compilation wins for a rebuilt file](../rationale/duplicate-latest-flags-win.md)

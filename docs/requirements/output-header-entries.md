---
title: Header file entries in the output
status: implemented
---

## Intent

Editors and linters often look up compile flags for a header file, not
only for translation units, so that features like "jump to definition"
work when a header is open. The user can opt in to a synthesized entry
for each header, with flags taken from a compiled translation unit. It
is off by default.

## Acceptance criteria

- **Given** a build that compiles `src/main.c`, next to an uncompiled
  `src/util.h`, **when** Bear finishes with header entries off (the
  default), **then** the database holds an entry for `src/main.c` only.
- **Given** the same build, **when** Bear finishes with header entries
  on and the default strategy, which considers headers in the same
  directory as a compiled source, **then** the database also holds an
  entry for `src/util.h`. It copies the arguments of `src/main.c` with
  the source path replaced by the header path and the output flag
  removed, and it has no `output` field. For example
  `["cc", "-c", "src/main.c", "-o", "src/main.o"]` gives
  `["cc", "-c", "src/util.h"]`.
- **Given** a build whose compilation emitted a dependency file that
  lists headers inside the compilation's working directory and system
  headers outside it, **when** Bear finishes with the dependency-file
  strategy, **then** the database holds a synthesized entry for exactly
  the listed headers inside the working directory, and no others.
- **Given** a build that runs `swiftc -c thing.swift`, next to an
  uncompiled `util.h`, **when** Bear finishes with header entries on,
  **then** the database holds no entry for `util.h`. Only C, C++, and
  Objective-C translation units give their arguments to a header entry.
- **Given** a build that compiles `src/util.h` directly
  (`cc -c src/util.h`) and `src/main.c` next to it, **when** Bear
  finishes with header entries on and the default duplicate detection,
  **then** the database holds exactly one entry for `src/util.h`: the
  real one from the build.
- A synthesized entry never displaces a real one. It is dropped when the
  key of duplicate detection (owned by `output-duplicate-detection`)
  matches a real entry; with a key or path format under which the two
  differ, both appear. Synthesized entries are validated like any other
  entry (owned by `output-json-compilation-database`).
- The set of file extensions that count as headers is built in. No
  configuration changes it.

## Rationale

- [Header entries: clone a sibling by default, read dependency files on request](../rationale/header-synthesis-sibling-clone.md)

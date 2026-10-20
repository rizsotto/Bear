---
title: Generated-file exclusion by filename pattern
status: implemented
---

## Intent

Machine-generated sources, such as Qt `moc` output or protobuf stubs,
often should not appear in the database, so that linters and editors
act only on hand-written code. A user can list filename patterns that
exclude, or re-include, matching sources. This is off by default.

## Acceptance criteria

- **Given** events that compile `main.cpp` and `moc_window.cpp`, and a
  rule that excludes the pattern `moc_*.cpp`, **when** Bear runs,
  **then** the database holds `main.cpp` only.
- **Given** the same events and no pattern rules, **when** Bear runs,
  **then** the database holds both files.
- **Given** events that compile `src/main.cpp` and `generated/main.cpp`,
  and a rule that excludes `generated/*.cpp`, **when** Bear runs,
  **then** the database holds `src/main.cpp` only. A pattern with a path
  separator matches the full source path as written in the entry. A
  pattern without one matches the basename.
- **Given** events that compile `main.cpp` and `util.cpp`, and a rule
  that excludes `*.cpp` followed by one that includes `main.cpp`,
  **when** Bear runs, **then** the database holds `main.cpp` only. The
  last matching rule wins, as for directory rules
  (`output-source-directory-filter`). A file that no pattern matches is
  kept.
- **Given** events that compile `src/main.cpp` and `src/moc_window.cpp`,
  a directory rule that includes `src`, and a pattern rule that
  excludes `moc_*.cpp`, **when** Bear runs, **then** the database holds
  `src/main.cpp` only. An entry is written only when both the directory
  rules and the pattern rules accept it.
- **Given** a configuration with a malformed pattern, **when** the
  configuration is validated, **then** validation fails with an error
  that names the rule.

## Rationale

- [Generated-file filtering uses user-supplied globs](../rationale/generated-file-filter-globs.md)

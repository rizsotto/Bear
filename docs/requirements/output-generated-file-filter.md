---
title: Generated-file exclusion by filename pattern
status: in-progress
---

## Intent

Machine-generated sources - Qt `moc` output, protobuf stubs, and the output of
other code generators - often should not appear in the compilation database, so
that linters and editors act only on hand-written code. A user can list
filename patterns that exclude (or re-include) matching source files. This is
off by default: with no such configuration the output is unchanged.

## Acceptance criteria

- A configuration option lists filename patterns, each with an include or
  exclude action. An entry whose source file matches an exclude pattern is
  omitted from the database.
- A pattern without a path separator matches the source file's basename; a
  pattern containing a separator matches the full source path as it appears in
  the entry.
- Rules are evaluated in order and the last matching rule wins, the same
  precedence the directory rules use. A file matched by no pattern rule is
  included.
- File-pattern rules and directory rules compose: an entry is emitted only when
  both the directory rules and the file-pattern rules accept it.
- An invalid pattern is rejected during configuration validation with an error
  that identifies the offending rule.
- With no file-pattern rules configured, every entry the directory rules accept
  is emitted; behaviour is unchanged.

## Non-functional constraints

Patterns are compiled once, when the output pipeline is built, not per entry.

## Testing

- Given events compiling `main.cpp` and `moc_window.cpp`, and a configuration
  excluding the basename pattern for `moc_*.cpp`, when bear runs, then the
  database contains `main.cpp` only.
- Given the same events and no file-pattern configuration, when bear runs, then
  the database contains both files.
- Given a rule whose pattern contains a path separator, when bear runs, then the
  pattern is matched against the full source path rather than the basename.
- Given a configuration containing a malformed pattern, when the configuration
  is validated, then validation fails with an error identifying the rule.

## Rationale

- [Generated-file filtering uses user-supplied globs](../rationale/generated-file-filter-globs.md)

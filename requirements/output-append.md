---
title: Append mode for compilation database
status: implemented
---

## Intent

When the user performs incremental builds or builds separate components at
different times, they need to accumulate compilation entries across multiple
Bear runs into a single compilation database. The `--append` flag merges new
entries with an existing `compile_commands.json` instead of overwriting it.

## Acceptance criteria

- When `--append` is specified and the output file exists, existing entries
  are preserved and new entries are added after them
- When `--append` is specified and the output file does not exist, Bear logs
  a warning and writes only the new entries (no error)
- When `--append` is not specified, the output file is overwritten with only
  the new entries (default behavior)
- When the existing file cannot be opened (e.g. permission denied), Bear
  returns an error and does not write output
- When the existing file opens but contains invalid JSON or invalid entries,
  Bear skips invalid entries individually with a logged warning per entry
  and preserves valid entries
- Existing entries appear before new entries in the combined output
- The combined output (existing + new) passes through the rest of the output
  pipeline (duplicate filtering, source filtering, atomic write)

## Implementation details

The `--append` flag is available both in the `semantic` subcommand and in
the combined mode (`bear --append -- <build>`).

When append mode is active, Bear reads the existing compilation database
before writing the new one. The existing entries are chained before the new
entries. This ordering matters: the duplicate filter (`output-duplicate-detection`) keeps the
first occurrence, so when a file is recompiled with identical flags the
original entry from the existing database is preserved.

The append step runs after entry conversion and before the atomic write
(`output-atomic-write`). The duplicate filter (`output-duplicate-detection`) and source filter run after
the atomic write stage in the pipeline.

Reading errors are handled at two levels:
- File-open failures (missing permissions, IO errors) propagate as hard
  errors. Bear does not write output in this case.
- Parse-level failures (malformed JSON entries) are handled per-entry: each
  invalid entry is skipped with a warning, and valid entries are preserved.
  For a wholly corrupted (non-JSON) file, the parser may yield zero entries
  and zero warnings -- the user receives no visible warning in this case.

## Non-functional constraints

- Must not corrupt the output file if Bear is interrupted during the read
  phase (the atomic write in `output-atomic-write` handles this)
- The existing database is read via a streaming iterator; however the
  underlying JSON parser may buffer the full array in memory

## Testing

Given no existing `compile_commands.json`:

> When the user runs `bear --append -- <compiler> -c file1.c`,
> then `compile_commands.json` is created with one entry for file1.c.

Given an existing `compile_commands.json` with an entry for file1.c:

> When the user runs `bear --append -- <compiler> -c file2.c`,
> then `compile_commands.json` contains entries for both file1.c and file2.c.

Given an existing `compile_commands.json` with an entry for file1.c:

> When the user runs `bear -- <compiler> -c file2.c` (no `--append`),
> then `compile_commands.json` contains only the entry for file2.c.

Given an existing `compile_commands.json` with corrupted JSON content:

> When the user runs `bear --append -- <compiler> -c file1.c`,
> then `compile_commands.json` contains only the entry for file1.c.

Given an existing `compile_commands.json` with some valid and some invalid entries:

> When the user runs `bear --append -- <compiler> -c new.c`,
> then the valid existing entries are preserved,
> the invalid entries are skipped with per-entry warnings,
> and the new entry is added.

Given an existing `compile_commands.json` with read permission denied:

> When the user runs `bear --append -- <compiler> -c file1.c`,
> then Bear exits with an IO error and does not write output.

Given an existing `compile_commands.json` and a new build that produces zero
compiler invocations:

> When the user runs `bear --append -- true`,
> then the existing entries are preserved unchanged.

Given an existing `compile_commands.json` with an entry for file1.c compiled
with `-O2`, and a new build that compiles file1.c with identical flags:

> When the user runs `bear --append -- <compiler> -c -O2 file1.c`,
> then the duplicate filter determines whether both entries survive,
> and the original entry (from the existing database) takes priority.

## Notes

- GitHub issue #532 reported severe performance degradation with `--append`
  on large projects in the old C++ implementation. The current Rust
  implementation uses iterators but the underlying JSON parser may still
  buffer the full file.
- GitHub PR #497 introduced an `--update` concept where existing entries
  with matching filenames are replaced rather than appended. This is related
  but distinct from basic append behavior.

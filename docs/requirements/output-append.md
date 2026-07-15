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

- When `--append` is specified and the output file exists, new entries are
  emitted first and the existing entries follow
- When `--append` is specified and the output file does not exist, Bear logs
  a warning and writes only the new entries (no error)
- When `--append` is not specified, the output file is overwritten with only
  the new entries (default behavior)
- When the existing file cannot be opened (e.g. permission denied), Bear
  returns an error and does not write output
- When the existing file opens but contains invalid JSON or invalid entries,
  Bear skips invalid entries individually with a logged warning per entry
  and preserves valid entries
- The combined output (new + existing) passes through the rest of the output
  pipeline (duplicate filtering, source filtering, atomic write); which entry
  survives when a file appears in both is owned by `output-duplicate-detection`

## Non-functional constraints

- Must not corrupt the output file if Bear is interrupted during the read
  phase (the atomic write in `output-atomic-write` handles this)

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

## Rationale

- [Latest compilation wins for a rebuilt file](../rationale/duplicate-latest-flags-win.md)

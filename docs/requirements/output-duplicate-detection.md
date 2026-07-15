---
title: Duplicate entry detection and filtering
status: implemented
---

## Intent

Build systems may invoke the compiler for the same source file more than
once - parallel make retries, ccache wrappers, or repeated builds that
append to an existing database after a file's flags change. The compilation database
specification (<https://clang.llvm.org/docs/JSONCompilationDatabase.html>)
allows multiple entries for the same file, noting this is for "different
configurations", but stale duplicates confuse downstream tools and let old
flags linger after a rebuild. By default Bear keeps a single entry per
source file (identified by its directory and path), so recompiling a file
with new flags updates its entry instead of accumulating a stale one. Users
who genuinely need several configurations recorded for one file can widen
the set of fields that distinguish entries.

## Acceptance criteria

- Duplicate entries are detected and only the first occurrence is kept
- Duplicate detection is based on configurable fields (default: `directory`
  and `file`)
- By default two entries for the same source file in the same directory are
  duplicates regardless of their compiler arguments, so only one survives;
  distinguishing entries by their flags requires adding the arguments field
  to the configured set
- Two entries are considered duplicates when all configured fields match
- Entries that differ in any configured field are preserved as distinct
- The first-occurrence guarantee combines with append ordering
  (`output-append`): a newly generated entry is emitted before the matching
  entry from the existing database, so the new entry wins and its flags
  replace the old ones
- When preload interception records both a compiler driver or wrapper and
  the child compiler it spawns for the same source, they collapse to a
  single entry under the default match; the driver or wrapper invocation
  survives because it is emitted first in the event stream
- Accepted entries appear in the output in the same order they were received
- Configuration validation rejects:
  - Empty field lists
  - Duplicate fields in the list
  - Both `command` and `arguments` in the same list (they are alternative
    representations of the same data)

## Non-functional constraints

- A duplicate is detected no matter how far apart the two occurrences
  are in the input

## Testing

Given a build that compiles file.c twice with identical flags:

> When Bear generates the compilation database,
> then only one entry for file.c appears in the output.

Given a build that compiles file.c with `-O2` and then with `-O3`:

> When Bear generates the compilation database with default duplicate config,
> then only one entry for file.c appears
> (default matching is `directory` and `file`, so arguments are ignored).

Given duplicate detection configured with `match_on: [directory, file, arguments]`
and a build that compiles file.c with `-O2` and then with `-O3`:

> When Bear generates the compilation database,
> then both entries appear (arguments are part of the match, so they differ).

Given files `src/util.c` and `lib/util.c` (same basename, different directories):

> When Bear generates the compilation database,
> then both entries are preserved (different directory means not a duplicate).
> Regression guard for issue #667.

Given duplicate detection configured with `match_on: [file]`:

> When a build compiles file.c twice with different flags,
> then only the first entry is kept (matching on file alone).

Given duplicate detection configured with `match_on: [file, output]`:

> When file.c is compiled to both `debug/file.o` and `release/file.o`,
> then both entries are preserved (different output paths).

Given duplicate detection configured with `match_on: [command, arguments]`:

> Then configuration validation rejects it
> with an error explaining the conflict.

Given duplicate detection configured with `match_on: []`:

> Then configuration validation rejects it
> with an error explaining the empty field list.

Given an `--append` run where file.c exists in the old database, and the
new build compiles file.c with different flags:

> When Bear generates the output,
> then only one entry for file.c appears, recording the new flags
> (the new entry wins, because new entries come first and default matching
> ignores arguments).

Given a compiler driver or wrapper and the child compiler it spawns are
both intercepted in preload mode, producing two events for one source:

> When Bear generates the compilation database with default duplicate
> detection,
> then exactly one entry survives for that source,
> and it records the driver/wrapper invocation (the driver event is
> emitted first, so it wins under first-seen matching).
> Regression guard for issue #638.

## Notes

- Compiler drivers and wrappers (MPI wrappers, Emscripten's `emcc`/`em++`,
  AMD's `hipcc`/`amdclang`/`amdflang`, ...) exec a child compiler that
  preload mode intercepts as well, so one compilation reaches this filter
  as two events for the same source. First-seen matching keeps the
  user-facing driver invocation, which is emitted before its child. The
  recognition requirement (`recognition-compiler-names`) relies on this
  behaviour rather than restating it.

## Rationale

- [Latest compilation wins for a rebuilt file](../rationale/duplicate-latest-flags-win.md)

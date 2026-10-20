# Latest compilation wins for a rebuilt file

## Context

Bear is often run with `--append` for partial or incremental builds:
each run adds the freshly compiled files to an existing
`compile_commands.json`. GitHub discussion
[#712](https://github.com/rizsotto/Bear/discussions/712) reported that when a
file's compiler flags change between runs, the database ends up with two
entries for that file - the stale one from the previous run and the new
one - because the two behaviours in place at the time worked against the
user:

- Default duplicate detection matched on `directory`, `file` and
  `arguments`. Changed flags mean different arguments, so the new entry
  was not seen as a duplicate and both were kept.
- Append emitted the existing database's entries first, so even when the
  flags were identical the first-occurrence rule kept the old entry.

The build system could not force a clean rebuild, and deleting
`compile_commands.json` before an append run defeats the point of
appending. Two options were on the table:

- **Opt-in `--update` flag** (the reporter's original request, and the
  shape of the withdrawn
  [PR #497](https://github.com/rizsotto/Bear/pull/497)): keep the current defaults and
  add a flag that replaces matching entries. Preserves today's behaviour
  for everyone, but every affected user has to discover and set it, and
  the stale-duplicate default is rarely what anyone actually wants.
- **Change the default**: drop `arguments` from the default match set and
  emit new entries before existing ones, so the newest invocation of a
  file wins automatically, no flag required.

## Decision

Change the default. Duplicate detection defaults to matching on
`directory` and `file` only, and `--append` emits new entries ahead of
the existing ones. Together with the existing first-occurrence rule, a
file rebuilt with new flags has its entry replaced rather than
duplicated. Recording several configurations for one file - the clang
spec's stated reason for allowing repeats - remains available by adding
`arguments` back to `duplicates.match_on`.

## Consequences

Incremental `--append` builds self-heal when flags change: no manual
deletion, no extra flag, and the database always reflects the latest
build. The costs:

- Within a single build, a file compiled twice from the same directory
  with different flags now collapses to one entry (the first seen), where
  before both were kept. This is rare and recoverable by widening
  `duplicates.match_on`.
- Append output ordering changes (new entries first). Tools that assume
  insertion order may notice, though the compilation database format does
  not promise an order.

Revisit if keeping per-configuration entries by default becomes a real
need - that would argue for a distinct output mode rather than reverting
the dedup default.

## References

- [output-duplicate-detection](../requirements/output-duplicate-detection.md), a requirement this supports
- [output-append](../requirements/output-append.md), a requirement this supports
- [#712](https://github.com/rizsotto/Bear/discussions/712), the discussion that reported the stale duplicates
- [PR #497](https://github.com/rizsotto/Bear/pull/497), the withdrawn opt-in `--update` flag

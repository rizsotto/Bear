---
title: Duplicate entry detection and filtering
status: implemented
tests:
  - duplicate_filter_config
---

## Intent

Build systems may invoke the same compiler command multiple times for the
same source file (e.g. parallel make retries, ccache wrappers, or repeated
builds with `--append`). The compilation database specification
(<https://clang.llvm.org/docs/JSONCompilationDatabase.html>) allows multiple
entries for the same file but notes this is for "different configurations."
Bear filters out true duplicates to keep the output clean and reduce
downstream tool confusion.

## Acceptance criteria

- Duplicate entries are detected and only the first occurrence is kept
- The first-occurrence guarantee means that in append mode (`output-append`),
  the original entry from the existing database takes priority over a new
  entry with identical fields
- Accepted entries appear in the output in the same order they were received
- Duplicate detection is based on configurable fields (default: `directory`,
  `file`, `arguments`)
- Two entries are considered duplicates when all configured fields match
- Entries that differ in any configured field are preserved as distinct
- The set of fields used for matching is configurable via the `duplicates`
  section in the configuration file
- Configuration validation rejects:
  - Empty field lists
  - Duplicate fields in the list
  - Both `command` and `arguments` in the same list (they are alternative
    representations of the same data)

## Implementation details

Bear uses a hash-based approach for duplicate detection. For each entry, a
hash is computed over the configured fields using Rust's `DefaultHasher`
(SipHash). The hash is checked against a set of previously seen hashes. If
new, the entry is accepted; if already seen, it is rejected.

The hash set grows with the number of unique entries (O(n) memory), but
entries are processed one at a time without buffering the full stream.

Hash collisions are theoretically possible. A collision would silently drop
a non-duplicate entry (false positive). With a 64-bit hash this is
extremely unlikely for typical compilation databases (thousands of entries)
but the probability grows with database size. This is an accepted trade-off
for simplicity and performance.

Duplicate detection operates on entries after path formatting (`output-path-format`).
This means the configured path format affects which entries are considered
duplicates. For example, two entries with different relative paths that
resolve to the same absolute path would only be detected as duplicates if
`absolute` or `canonical` path formatting is active.

The duplicate filter runs after the source filter and before final
serialization. It processes the combined stream of existing and new entries
when append mode (`output-append`) is active.

### Configurable fields

The following fields from the compilation database entry can be used for
duplicate matching (see `output-json-compilation-database` for field definitions):

| Field | Config name | Description |
|---|---|---|
| `directory` | `directory` | Working directory of the compilation |
| `file` | `file` | Source file path |
| `arguments` | `arguments` | Argument array (mutually exclusive with `command`) |
| `command` | `command` | Command string (mutually exclusive with `arguments`) |
| `output` | `output` | Output file path |

### Default configuration

```yaml
duplicates:
  match_on:
    - directory
    - file
    - arguments
```

This means two entries are duplicates only if they have the same working
directory, the same source file, and the same compiler arguments.

## Non-functional constraints

- Hash-based detection uses O(n) memory proportional to unique entries
- The filter processes entries one at a time without buffering the full
  stream, but retains hashes for all unique entries seen so far

## Testing

Given a build that compiles file.c twice with identical flags:

> When Bear generates the compilation database,
> then only one entry for file.c appears in the output.

Given a build that compiles file.c with `-O2` and then with `-O3`:

> When Bear generates the compilation database with default duplicate config,
> then both entries appear (different arguments means not a duplicate).

Given files `src/util.c` and `lib/util.c` (same basename, different directories):

> When Bear generates the compilation database,
> then both entries are preserved (different directory means not a duplicate).

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
new build also compiles file.c with the same flags:

> When Bear generates the output,
> then only one entry for file.c appears
> (the original from the old database, because existing entries come first).

## Notes

- GitHub issue #667 reported that files with identical basenames in separate
  directories were incorrectly dropped. This was caused by matching on
  filename alone without considering the directory. The default config
  includes both `directory` and `file` to prevent this.
- GitHub issue #638 reported duplicate entries from clang's internal `-cc1`
  frontend invocations. These are filtered by the semantic analyzer before
  reaching the duplicate filter, but the duplicate filter provides a safety
  net.
- GitHub PR #497 introduced an `--update` concept where duplicates are
  replaced rather than dropped. This is not currently implemented in the
  Rust version but the configurable field matching provides a foundation
  for it.

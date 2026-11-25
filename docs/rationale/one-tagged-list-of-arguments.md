# One tagged list of arguments

## Context

A call's flags do not all come from argv. A response file adds tokens
where `@file` stood. Some environment variables add words before argv
(MSVC `CL`) or after it (`CPATH`, MSVC `_CL_`). The interpretation must
keep these in output order, and tools that show a call want to say where
each item came from.

Not every argument that starts with `@` is a response file. On macOS
`-install_name @rpath/libfoo.dylib` and `-rpath @loader_path/..` are
common, and the `@` part is a flag's value.

Options considered:

- Splice every `@` argument's file into argv before parsing, as GCC
  does. A flag in the file can then take its value from argv. But it also
  tries to read flag values like `@rpath/libfoo.dylib`, and with inlining
  on by default every such call logs a warning about a missing file.
  Build tools put whole flags in a response file, so the exact pairing
  buys little.
- Keep read files in a map keyed by path. Every view of the call needs a
  lookup, and one path can mean two things, before and after `/link`.
- Keep separate per-variable environment lists next to the main list.
  This treats two kinds of extra tokens in two ways.
- Tag each token with its origin. It is exact, but nothing needs that.

## Decision

Argv is classified first, so only an `@file` at a token position counts.
Then each such file is read, its tokens are classified on their own with
the same compiler family, and the items take the place of the `@file`.
Nested files are read the same way, up to a depth limit. The
interpretation keeps one list of items in output order. Each item is
tagged with where it came from: the command line, a response file, or an
environment variable. Environment words are placed once, some before
argv and some after, and carry their variable name.

An `@file` that is not read (inlining off, file missing, depth limit,
input from another machine) stays an item in every compiler family.

## Consequences

Flag values are never read as files, and views of the call need no
lookup. A flag split across the file boundary is not paired, and a
`/link` inside a file does not reach the argv after it. The requirement
lists both as known limitations.

## References

- [output-response-file-inlining](../requirements/output-response-file-inlining.md), the requirement this supports

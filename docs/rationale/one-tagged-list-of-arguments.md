# One tagged list of arguments

## Context

A call's flags do not all come from argv. A response file adds tokens
where `@file` stood. Some environment variables add words before argv
(MSVC `CL`) or after it (`CPATH`, MSVC `_CL_`). The interpretation must
keep these in output order, and tools that show a call want to say where
each item came from.

GCC expands `@file` before it parses options. A flag in the file can take
its value from argv, and the reverse.

Options considered:

- Parse each response file apart from argv and keep the items in a map
  keyed by path. This adds a boundary GCC does not have. A flag at the
  end of the file loses its value. `/link` inside a file does not affect
  the argv after it. A path can mean two things, before and after
  `/link`. Every view of the call needs a lookup into the map.
- Keep separate per-variable environment lists next to the main list.
  This treats two kinds of extra tokens in two ways.
- Tag each token with its origin. It is exact, but nothing needs that.

## Decision

A read response file is spliced into the token stream where the `@file`
stood, and parsed with argv as one stream. The interpretation keeps one
list of items in output order. Each item is tagged with where it came
from: the command line, a response file, or an environment variable.
Environment words are placed once, some before argv and some after, and
carry their variable name.

An `@file` that is not read (inlining off, file missing, depth limit,
input from another machine) stays an item in every compiler family.

## Consequences

Views of the call need no lookup, and edge cases that cross a file
boundary behave as in GCC. An item whose flag and value come from
different places shows one origin. The output is still right.

## References

- [output-response-file-inlining](../requirements/output-response-file-inlining.md), the requirement this supports

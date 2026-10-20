---
title: Exit codes by mode
status: implemented
---

## Intent

Shells, CI runners, and parent `make` rules read Bear's exit code to
decide whether a run worked. When Bear runs a build, the exit code
reports the build's result. When Bear runs without a build, it reports
whether Bear could do its own job. This file owns the whole exit-code
contract, so a caller never has to piece it together from the mode
requirements.

## Acceptance criteria

| Mode | Outcome | Exit code |
|---|---|---|
| any | help is requested | `0` |
| any | the command line is malformed | non-zero, before any work runs |
| any | an error of Bear's own (an input cannot be read, the output cannot be created or written, the build cannot start) | non-zero |
| combined, intercept | the build exits on its own | the build's code |
| combined, intercept | the build is killed by a signal | non-zero |
| combined, intercept | entries are dropped by output validation | the build's code |
| semantic, parse-sh | the input is empty, or is understood even when the output is empty | `0` |
| semantic, parse-sh | non-empty input in which nothing is understood | non-zero |

Which command lines are malformed is owned by the mode requirements, for
example [`interception-events-format`](interception-events-format.md)
for the event-file usage errors.

- **Given** a build that exits `0`, **when** it runs under Bear in
  combined mode, or in intercept mode with an output file, **then** Bear
  exits `0`.
- **Given** a build that exits `42`, **when** it runs under Bear in
  either mode, **then** Bear exits `42`. Codes in the portable range
  (0-255 on Unix) pass through unchanged.
- **Given** a build killed by a signal, so it has no exit code, **when**
  Bear finishes, **then** Bear exits non-zero, never `0`. How the signal
  reaches the build is owned by
  [`cli-signal-forwarding`](cli-signal-forwarding.md).
- **Given** a build that handles a forwarded signal and exits on its own,
  **when** Bear finishes, **then** Bear exits with the build's code, `0`
  included.
- **Given** a build that exits `0` and an output destination that becomes
  unwritable during the build, so the final rename fails, **when** Bear
  finishes, **then** Bear exits non-zero although the build succeeded.
  The write itself is owned by [`output-atomic-write`](output-atomic-write.md).
- **Given** a build that exits `0` but where every candidate entry fails
  output validation, **when** Bear finishes, **then** the database is an
  empty array and Bear exits `0`. The validation rules are owned by
  [`output-json-compilation-database`](output-json-compilation-database.md).
- **Given** semantic mode reading a conforming events file, **when** it
  runs, **then** Bear exits `0`. This holds when every event is
  understood but yields no valid entry (an event whose working directory
  is empty), and the database is an empty array.
- **Given** semantic mode reading non-empty input in which every line is
  non-conforming, **when** it runs, **then** Bear exits non-zero.
- **Given** parse-sh mode reading shell text with at least one line in
  the supported subset, **when** it runs, **then** Bear exits `0`. This
  holds when the text names no compiler (a lone `mv` line), and the
  database is empty.
- **Given** parse-sh mode reading non-empty input in which every line is
  an unsupported construct, **when** it runs, **then** Bear exits
  non-zero.
- **Given** semantic mode or parse-sh mode with a named input file that
  does not exist, **when** it runs, **then** Bear exits non-zero.
- **Given** a command line with no build command or with an unknown
  argument, **when** the user runs Bear, **then** it exits non-zero and
  prints the usage or the error before running anything.
- **Given** a request for help, for Bear or for one of its modes,
  **when** the user runs it, **then** Bear prints the usage and exits
  `0`.

The skip-and-continue rule for input lines is owned by
[`interception-events-format`](interception-events-format.md).

## Known limitations

- Bear does not encode the signal that killed a build in its exit code
  (no `128 + signal number`), so a caller that reads only the exit code
  cannot tell a killed build from a failed one. The build still gets the
  real signal.
- Nested Bear runs (Bear running under another Bear) are not supported,
  and their exit code is undefined. The exclusion is owned by
  [`cli-signal-forwarding`](cli-signal-forwarding.md).

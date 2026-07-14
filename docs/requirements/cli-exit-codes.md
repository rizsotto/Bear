---
title: Exit codes by mode
status: implemented
---

## Intent

Shells, CI runners, and parent `make` rules read Bear's exit code to
decide whether a run succeeded. Bear runs in several modes, and what the
exit code *means* differs by mode: a mode that runs a build reports the
build's fate, while a mode that only transforms data reports whether Bear
could do that transformation. This requirement collects the whole
exit-code contract in one place, so a caller never has to reconstruct it
from the individual mode requirements.

The governing principle is one line: when Bear runs a build it proxies the
build's outcome; when Bear runs standalone it reports whether it could do
its own job. Everything below follows from that.

## Acceptance criteria

Common to all modes:

- Requesting help exits `0`.
- A malformed invocation -- no build command given, an unknown argument, a
  build-running mode asked to write the database to a stream, or
  interception with no event file named -- exits non-zero and is reported
  before any work runs.
- A run terminated by a signal never exits `0`. In build-running modes the
  signal is first relayed to the build; delivery and process-tree teardown
  are governed by [`cli-signal-forwarding`](cli-signal-forwarding.md).
- A genuine I/O failure that prevents Bear from writing its output exits
  non-zero, even when the work Bear wrapped or transformed otherwise
  succeeded.

Build-running modes (`bear -- <build>`, `bear intercept -- <build>`):

- When the build exits normally, Bear's exit code equals the build's:
  `0` for success is preserved, and any non-zero code is preserved. Codes
  in the portable range (0-255 on Unix) are propagated byte-for-byte.
- Data-quality outcomes never change the exit code. Entries dropped during
  output validation -- even when every entry is dropped and the database is
  written empty -- leave the exit code reflecting the build alone (see
  [`output-json-compilation-database`](output-json-compilation-database.md)).

Standalone filter modes (`bear semantic`, `bear parse-sh`):

- A run that understands its input and produces its output succeeds with
  `0`, even when the output is empty because the input yielded nothing
  (semantic: every candidate entry was dropped by validation; parse-sh: the
  input named no commands).
- Non-empty input in which nothing could be understood exits non-zero
  (semantic: no conforming event was accepted; parse-sh: every line was
  skipped). See [`interception-events-format`](interception-events-format.md)
  for the shared skip-and-continue rule.

## Known limitations

- The signal that terminated a build is not encoded in Bear's exit code:
  the `128 + signal_number` shell convention is not followed, so a caller
  inspecting only Bear's exit code cannot distinguish signal termination
  from an ordinary build failure. The build itself still receives the real
  signal; see [`cli-signal-forwarding`](cli-signal-forwarding.md).

## Testing

Given a build that exits successfully:

> When the user runs `bear -- true`, then `bear` exits `0`; likewise
> `bear intercept -- true` exits `0`.

Given a build that exits with a non-zero code:

> When the user runs `bear -- false`, then `bear` exits non-zero, matching
> the build; likewise `bear intercept -- false`.

Given a build stopped by a signal:

> When a termination signal is sent to `bear` while the build runs, then
> `bear` exits non-zero.

Given a build that compiles successfully but whose database cannot be
written (the output path exists as a directory):

> When the user runs the build under `bear`, then `bear` exits non-zero
> even though the build itself succeeded.

Given a `semantic` run over a conforming events file:

> When the user runs `bear semantic --input events.json`, then `bear`
> exits `0`.

Given a `semantic` run over non-empty input in which every line is
non-conforming:

> When the user runs `bear semantic` on it, then `bear` exits non-zero.

Given a `parse-sh` run over shell text with at least one recognizable
command:

> When the user runs `bear parse-sh` on it, then `bear` exits `0`.

Given a `parse-sh` run over non-empty input in which every line is an
unsupported construct:

> When the user runs `bear parse-sh` on it, then `bear` exits non-zero.

Given a filter run whose named input file does not exist:

> When the user runs `bear semantic --input nonexistent.json` (or
> `bear parse-sh` on a missing file), then `bear` exits non-zero.

Given a malformed invocation:

> When the user runs `bear` with no build command, or with an unknown
> argument, then `bear` exits non-zero and prints usage or the specific
> error before running anything.

Given a help request:

> When the user runs `bear --help` (or a subcommand's `--help`), then
> `bear` exits `0` and prints usage.

# No implicit event-file name: intercept names it, semantic reads stdin

## Context

Through Bear 4.1.x, `bear intercept` wrote its event stream to
`events.json` by default and `bear semantic` read `events.json` by
default. The paired defaults made the split two-step workflow flagless,
at the price of a magic rendezvous filename in the current working
directory: the two subcommands coordinated through a name neither the
user chose nor any external tool consumes. That carried a real hazard -
a stale `events.json` from an earlier run could be silently reprocessed
by a bare `bear semantic` - and a hidden coupling: nothing in either
invocation said the two commands were connected.

Shipping `bear parse-sh` in 4.2.0 changed the balance. The pipe
`bear parse-sh | bear semantic` becomes a primary way to feed
`semantic`, and the `events.json` default fought it: forgetting
`--input -` died with a baffling "Event file not found: events.json".
Viewed as a UNIX tool, `semantic`
is a pure filter - no build, stdout free - and filters read standard
input when no file is named. `intercept` is the opposite case: it can
never use standard output (the build owns that stream), so no natural
default exists for it at all.

The two file names in play are not equivalent. `compile_commands.json`
is an ecosystem contract - Clang tooling finds the database by that
exact name - so defaulting to it carries meaning. `events.json` was a
name Bear invented; no third-party contract depends on it.

Alternatives considered:

- Keep the paired defaults and only improve the missing-file error with
  a "reading from a pipe? pass --input -" hint. Rejected: it treats the
  symptom and keeps both the magic-filename coupling and the stale-file
  hazard.
- Default `semantic --input` to `-` only when stdin is not a TTY.
  Rejected: isatty-dependent semantics make the identical command line
  behave differently in a terminal and in CI, and CI stdin is often an
  empty pipe even when the user meant a file - a silent
  empty-database path.
- Give `intercept` a stream default by redirecting the build's own
  stdout to stderr, freeing standard output for the event stream.
  Rejected: it silently changes observable build behavior - tools that
  detect a tty or write results to stdout would break - for a pipeline
  no producer needs.
- Mandate `--input` on `semantic` too, symmetric with `intercept`.
  Rejected: a pure filter refusing to read stdin by default fights the
  convention this redesign invokes, permanently bakes `--input -` into
  the flagship parse-sh pipeline, and diverges from `parse-sh`, whose
  input already defaults to `-`. The asymmetry with `intercept` is
  principled: require the flag where no natural default exists; use the
  conventional default where one does.
- A positional file operand (`bear semantic [FILE]`, stdin when
  omitted), the purest filter spelling. Not taken: `-i/--input`
  defaulting to `-` is equally conventional and keeps `semantic`
  consistent with `parse-sh`'s existing flags.

## Decision

`bear intercept` requires an explicit `--output`; there is no default
event-file name. `bear semantic --input` defaults to `-`, reading the
event stream from standard input like any filter. The
`compile_commands.json` output default stays everywhere. Two guards
soften the filter footguns: `semantic` prints a stderr notice when it
is about to read events from a terminal, and warns when the event
stream turned out empty.

## Consequences

- The parse-sh pipeline is flagless: `make -n | bear parse-sh | bear
  semantic`. The confusing missing-file failure dissolves instead of
  being papered over with a hint.
- The stale-`events.json` hazard is gone; an event file is only ever
  read because the user named it.
- The old two-step habit breaks loudly and early: `bear intercept`
  without `--output` is a usage error before the build runs, the best
  available failure mode.
- Scripts written for 4.1.x split workflows must add `--output` /
  `--input`. This is a deliberate breaking change shipped in 4.2.0,
  before parse-sh examples ossify; it only gets more expensive later.
- A bare `bear semantic` on a terminal blocks reading stdin like `sort`
  or `jq` would; the TTY notice names the way out.

## References

- Requirement:
  [`interception-events-format`](../requirements/interception-events-format.md)
  (the seam whose endpoints this names).
- Related rationale:
  [`parse-sh-producer`](parse-sh-producer.md) (the producer that made
  the pipe a first-class feed).
- The footgun this supersedes: a bare `bear semantic` fed from a pipe
  failed with "Event file not found: events.json" instead of reading
  standard input.

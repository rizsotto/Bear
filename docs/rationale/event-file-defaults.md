# No implicit event-file name: intercept names it, semantic reads stdin

## Context

Through Bear 4.1.x, `bear intercept` wrote its event stream to
`events.json` by default and `bear semantic` read `events.json` by
default. The paired defaults made the split two-step workflow flagless.
The price was a magic rendezvous file in the current directory: the two
subcommands met at a name the user never chose and no outside tool
reads. A stale `events.json` from an earlier run could be silently
processed again by a bare `bear semantic`. Nothing in either command
line showed that the two were connected. Piping a producer into
`bear semantic` also failed, with "Event file not found: events.json",
unless the user passed `--input -`.

Seen as a UNIX tool, `semantic` is a pure filter: it runs no build, and
stdout is free. Filters read standard input when no file is named.
`intercept` is the opposite case. It can never use standard output,
because the build owns that stream, so it has no natural default at all.

The two file names are not equal. `compile_commands.json` is an
ecosystem contract: Clang tooling finds the database by that exact name,
so a default to it carries meaning. `events.json` was a name Bear made
up, and no outside contract depends on it.

Options considered:

- Keep the paired defaults and add a "reading from a pipe? pass
  --input -" hint to the missing-file error. Rejected: it treats the
  symptom and keeps both the coupling and the stale-file hazard.
- Default `semantic --input` to `-` only when stdin is not a TTY.
  Rejected: the same command line would behave differently in a
  terminal and in CI. CI stdin is often an empty pipe even when the user
  meant a file, which gives a silent empty database.
- Give `intercept` a stream default by sending the build's stdout to
  stderr. Rejected: it changes how the build behaves. Tools that detect a
  tty or write results to stdout would break.
- Require `--input` on `semantic` too, to match `intercept`. Rejected: a
  filter that will not read stdin by default fights the convention, and
  it would differ from `parse-sh`, whose input also defaults to `-`.
  Require the flag where no natural default exists; use the usual
  default where one does.
- A positional file operand (`bear semantic [FILE]`, stdin when
  omitted). Not taken: `-i/--input` defaulting to `-` is just as
  conventional and matches the `parse-sh` flags.

## Decision

`bear intercept` requires an explicit `--output`, and `-` is not
accepted. There is no default event-file name. `bear semantic --input`
defaults to `-` and reads the event stream from standard input. The
`compile_commands.json` output default stays everywhere. Two guards
soften the filter footguns: `semantic` warns on stderr when it is about
to read events from a terminal, and when the event stream is empty.

## Consequences

- Piping a producer into `bear semantic` needs no flags. The confusing
  missing-file failure goes away instead of getting a hint.
- The stale-`events.json` hazard is gone. An event file is only read
  because the user named it.
- The old two-step habit breaks loudly and early: `bear intercept`
  without `--output` is a usage error before the build runs.
- Scripts written for the 4.1.x split workflow must add `--output` and
  `--input`. This is a deliberate breaking change in 4.2.0. It only gets
  more expensive later.
- A bare `bear semantic` on a terminal blocks on stdin, as `sort` or
  `jq` would. The terminal warning names the way out.

## References

- [interception-events-format](../requirements/interception-events-format.md), the seam whose endpoints this names
- [parse-sh-producer](parse-sh-producer.md), the other mode whose input defaults to stdin

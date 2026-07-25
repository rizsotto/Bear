# The compilation database is parse-sh's only product

## Context

`parse-sh` first landed on the 4.2.0-rc branch as a pure filter: it
emitted the documented execution-event stream, and the documented usage
was a three-stage pipe, `make -n | bear parse-sh | bear semantic`. A
pre-release review questioned that shape. Every issue driving the
feature (#284, #287, #219, #644, #456, #404) asks for
`compile_commands.json`; none asks for the intermediate stream. Bear's
headline UX elsewhere is one command in, database out (`bear -- make`),
and forcing every dry-run user through the split pipeline exposed an
implementer's seam as the user interface.

Shapes considered for shortening the usage to `make -n | bear parse-sh`:

- An output-selection flag on `parse-sh` (database by default, events on
  request). Rejected: the flag would flip both the meaning and the
  default of the output destination, and would make configuration
  validity conditional on another flag - two personalities in one
  subcommand.
- A sibling subcommand pair: `parse-sh` producing the database,
  `parse-sh-events` keeping the filter. Statically clean - each command
  one personality - but rejected as speculative surface: no user has
  asked for the raw stream.
- The same sibling, hidden from help as a debug tool. Rejected: help,
  man page, and generated completions would diverge, and the audience
  for a debug tool is exactly the audience reading help.
- Keeping the stream at all was then examined on its own merits. The
  economics that justify the `intercept` / `semantic` split do not
  transfer: an intercepted event file is precious because re-running the
  build is expensive, while dry-run text is cheap to keep and re-parse,
  so caching the intermediate saves nothing. Debug visibility does not
  need it either - skip reporting is loud on stderr by default, and
  per-candidate debug logging can show what the parser reconstructed
  without a second output format.

The timing argument settled it: 4.2.0 was not yet released. Adding a
public events output later is a compatible addition; shipping one and
removing it later is a breaking change. Pre-release is the only moment
the minimal surface is free.

## Decision

`parse-sh` is a single subcommand whose only product is the compilation
database: shell text in (stdin by default), `compile_commands.json` out
(default destination), with configuration, append, and
standard-output streaming behaving as in the other database-producing
modes. The reconstructed executions remain the internal seam between
the parser and the unchanged semantic stage; they are never serialized
or exposed.

## Consequences

- The headline usage is one stage: `make -n | bear parse-sh`.
- Configuration now applies to `parse-sh` (it runs the semantic stage),
  inverting the earlier reject-config rule that guarded the pure
  filter.
- `bear intercept` is the only Bear producer of the events file. The
  interchange format keeps its documented value for third-party
  producers feeding `bear semantic`; what weakens is only the
  "second in-tree producer validates the seam" argument from
  [parse-sh-producer](parse-sh-producer.md) - the parser still targets
  the same execution model internally.
- If real demand for the raw stream appears (for example, link commands
  from dry-run text once linker support exists), a public events output
  can be added compatibly; its parsing semantics are already specified
  by the requirement.

## References

- Requirement:
  [`interception-shell-text-parsing`](../requirements/interception-shell-text-parsing.md)
  (the contract this reshapes).
- Prior rationale: [parse-sh-producer](parse-sh-producer.md) (the
  quarantined-producer decision this narrows),
  [event-file-defaults](event-file-defaults.md) (the stdio-default
  reasoning the earlier pipeline relied on).
- Issues: #284, #287, #219, #644, #456, #404.

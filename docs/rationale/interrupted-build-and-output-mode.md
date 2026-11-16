# An interrupted build and the output mode

## Context

A user presses Ctrl-C on a build under Bear. Bear has captured part of
the build. Two things can happen to the output: publish the partial
entries, or keep the old file. Neither is right for everyone.

"Interrupted" means Bear itself got a termination signal while the build
ran, relayed or not. A build killed by something else is a failed build
([failed-build-still-publishes](failed-build-still-publishes.md)).

The output mode already says how much the old file is worth. When
overwriting, the user said the old file does not matter. When appending,
the old file is part of the result. So the mode decides.

Options:

- Never publish an interrupted build. Rejected: it is the simplest rule,
  but it changes today's default, where an interrupted run publishes,
  and takes away the partial output that
  [#135](https://github.com/rizsotto/Bear/issues/135) asked for.
- The reverse: appending merges, overwriting keeps. Rejected: the
  default would change at once, and when appending becomes the default
  it would publish partial merges.
- Decide by the build's status alone (killed by a signal: keep; exited:
  publish). Rejected: ninja and most shell traps exit with a code on
  Ctrl-C, so Ctrl-C would still publish for them.
- Also keep the output when the build was killed by a signal with no
  signal to Bear. Rejected: that splits "failed build" in two.

## Decision

When appending, an interrupted build leaves the existing output as it
was. When overwriting, it publishes what was captured, as it does
today. Bear prints one warning line that says which one happened.

A build that exits `0` always publishes, in both modes. This keeps one
invariant a script can rely on: if Bear exits `0`, the output was
published. Without it, a Ctrl-C that the build ignored, or one that came
as the build ended, would give exit 0 and a stale database.

The rule covers an interrupt during the build only. An interrupt while
Bear writes the output, and any interrupt in semantic and parse-sh,
never publishes: the watcher removes the temporary file and dies
([abort-on-signal](abort-on-signal.md)). Publishing there would mean
finishing a half-written file.

## Consequences

- The default behavior does not change while overwriting is the default.
  When appending becomes the default, the safe behavior becomes the
  default by itself.
- When appending, Ctrl-C near the end of a long build gives no new
  entries. A CI job that hits its timeout loses its partial result too.
  The user can let the build fail, or overwrite.
- When overwriting, Ctrl-C replaces a full database with a partial one,
  as today. The warning line says so.

## References

- [output-append](../requirements/output-append.md), the requirement this supports
- [output-atomic-write](../requirements/output-atomic-write.md), the write that never publishes on an interrupt
- [cli-exit-codes](../requirements/cli-exit-codes.md), exit 0 means published
- [failed-build-still-publishes](failed-build-still-publishes.md), the failed build
- [abort-on-signal](abort-on-signal.md), an interrupt while Bear writes
- [append-default-migration](append-default-migration.md), the planned default
- [#135](https://github.com/rizsotto/Bear/issues/135), the request for partial output

# End like the build

## Context

When the build ends, Bear must report it. If the build exited, the exit
code is clear. If a signal killed the build, there is no code. Callers
read Bear's end: a shell script stops at Ctrl-C only when its child
really died by `SIGINT`, and make prints "Interrupt" for a recipe that
died by a signal.

Options:

- Exit 1, as before. Rejected: shells and make cannot see the interrupt.
- Exit `128 + N` for every signal. Rejected: bash stops a script only
  when the child really died by `SIGINT`.
- Re-raise only the signals that do not dump core, `128 + N` for the
  rest. Rejected: `SIGQUIT` (Ctrl-\) is one Bear relays, and a script
  around Bear would not see Bear die by it. Two rules where other tools
  have one.

Precedent: GNU `timeout`, `sudo`, and `strace` turn core dumps off and
re-raise the child's signal, with `128 + N` as fallback.

A re-raised core-dumping signal would make Bear dump core, which is
noise: the crash is in the build, not in Bear. On Linux `setrlimit` is
not enough. With a pipe core pattern (systemd-coredump) the wait status
of Bear still carried the core flag. `prctl(PR_SET_DUMPABLE, 0)` cleared
it. On macOS and BSD `setrlimit` is assumed to be enough.

## Decision

If the child exited, Bear exits with the same code. If a signal killed
the child, Bear turns its core dumps off, resets that signal to default,
unblocks it, and raises it, as the last thing it does, after all cleanup.
Only when the raise returns, for a signal the reset does not know, does
Bear exit with `128 + N`.

On Windows there is no such raise. Bear exits with the child's full exit
code, and with `0xC000013A` (what a console program stopped by Ctrl-C
returns) when no child ran.

Both binaries, the driver and the wrapper, use one helper.

## Consequences

- `bear -- sleep 3; echo NEXT` in a script stops at Ctrl-C like the
  plain `sleep` does.
- For a core-dumping signal the shell prints the signal name, as for the
  plain build, but not "(core dumped)". No core file is made for Bear.
  The build's own core dump is not affected.
- Turning off core dumps needs one small `unsafe` call per platform,
  gated by a probe, because the rules of the project forbid gating on
  the operating system name.
- The same ending holds when a signal aborts Bear with no build
  ([abort-on-signal](abort-on-signal.md)).

## References

- [cli-exit-codes](../requirements/cli-exit-codes.md), the requirement this supports
- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the signal rules
- [process-tree-teardown](process-tree-teardown.md), the relay that comes before the end
- [supervise-a-compiler-call](supervise-a-compiler-call.md), the wrapper ends the same way

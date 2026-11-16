# Supervise a compiler call

## Context

With the wrapper mechanism the chain is `bear-driver` -> `make` ->
`bear-wrapper` -> real `cc`. The wrapper reports the call, then runs the
real compiler. Bear does not supervise the build
([process-tree-teardown](process-tree-teardown.md)), and the wrapper is
a stand-in for the compiler in the same way. This entry covers only what
the wrapper does differently.

The force is the same as for the driver: a build runs the same with and
without Bear. make sends `SIGTERM` to its jobs by pid, and make is in
the compiler's process group. A wrapper that did not relay it would
leave the compiler running.

Options:

- Each wrapper makes its own process group. Rejected: the build splits
  into many groups, and the terminal's job control no longer reaches the
  compiler.
- Each wrapper runs its own grace window and `SIGKILL`. Rejected: Bear
  sends no signal of its own anywhere.
- A different relay rule for the wrapper. Rejected: two rules for one
  situation.
- The wrapper uses the same relay rule and the same exit as the driver.
  Chosen.

## Decision

The wrapper uses the same `supervise` and the same exit function as the
driver. It stays in the group it inherits, relays a termination signal
to its direct child by the rule of
[process-tree-teardown](process-tree-teardown.md), waits for the child
without a limit, and ends like the child
([end-like-the-build](end-like-the-build.md)).

A terminal Ctrl-C reaches the compiler from the terminal, and the
wrapper skips it. The `SIGTERM` from make is relayed, because a process
sent it.

The wrapper has no artifacts to clean, so it registers the signals right
before it starts the compiler. Before that point the default action
applies, which is what the compiler would have done.

## Consequences

- Under make, a signal to the group stops the compiler through the
  group. A signal to the wrapper's pid alone reaches the compiler
  through the relay.
- A compiler that ignores the signal keeps the wrapper waiting. The
  wrapper does not force it.
- The wrapper and the driver share one signal rule, so a change to it is
  one change.

## References

- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the requirement this supports
- [process-tree-teardown](process-tree-teardown.md), the rules the wrapper follows
- [end-like-the-build](end-like-the-build.md), how the wrapper ends
- [wrapper-mode-design-decisions](wrapper-mode-design-decisions.md), the wrapper mechanism itself

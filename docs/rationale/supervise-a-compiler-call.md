# Supervise a compiler call

## Context

With the wrapper mechanism the chain is `bear-driver` -> `make` ->
`bear-wrapper` -> real `cc`. The wrapper reports the call, then runs the
real compiler under the same `supervise()` the driver uses. So
supervision nests: the driver is the leader at the top, and each wrapper
is a supervisor of one compiler call further down. How the leader
starts, waits, forwards and kills is in
[process-tree-teardown](process-tree-teardown.md). This entry covers
only what the wrapper does differently.

The forces are the leader's: one signal to Bear must stop the whole tree
within the budget of
[cli-signal-forwarding](../requirements/cli-signal-forwarding.md), and
the build's own signal handling must keep working. Options:

- Each wrapper makes its own process group, like the leader. Rejected:
  the build splits into many groups. A top-level `killpg` then misses
  the processes below each wrapper, which reopens the hole grouping is
  meant to close.
- Each wrapper runs its own grace window and `SIGKILL`. Rejected: the
  leader's group kill (or `cgroup.kill`) is the real teardown, and it
  already reaches the wrapper's child. A second timer adds nothing.
- The wrapper stays in the group and cgroup it inherits, relays a signal
  to its direct child only, and waits. Chosen.

## Decision

The wrapper does not create a process group or a cgroup. On a signal it
relays the signal to its direct child, waits for it, and passes the
child's exit code back. The leader owns the only escalation timer.

## Consequences

- The wrapper and the compiler stay in the leader's process group, so
  the leader's forward and `SIGKILL` reach them with one `killpg`.
- The wrapper inherits the leader's cgroup through its parent, so one
  `cgroup.kill` from the leader reaches the whole tree.
- A compiler that ignores the signal is stopped by the leader when the
  grace window ends, not by the wrapper. The wrapper just waits until
  then.

## References

- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the requirement this supports
- [process-tree-teardown](process-tree-teardown.md), how the leader supervises the build
- [wrapper-mode-design-decisions](wrapper-mode-design-decisions.md), the wrapper mechanism itself

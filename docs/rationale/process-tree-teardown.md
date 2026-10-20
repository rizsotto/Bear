# Tear down the whole build tree: process group, then cgroup, with real-signal forwarding

## Context

When Bear supervises a build (`bear -- make`) and a termination signal
arrives, the whole process tree under the build must stop within the
budget that [cli-signal-forwarding](../requirements/cli-signal-forwarding.md)
sets. Bear should still write the partial `compile_commands.json` it has
collected. The first `supervise()` only sent `SIGKILL` to the direct
child. Grandchildren were reparented to init and kept running. And
`SIGKILL` cannot be trapped, so neither the build's own trap nor running
compilers got a chance to wind down.

Three mechanisms can stop a whole tree:

| Platform | Mechanism | Can a child escape? |
|---|---|---|
| any unix | process group (`setpgid`/`setsid`) + `killpg` | yes, by calling `setsid` itself |
| Linux | cgroup v2 `cgroup.kill` | no, unprivileged moves are denied |
| Windows | Job Object (`KILL_ON_JOB_CLOSE`) | no |

Process groups work on every unix and need no new dependency (`libc` is
already there). Their one gap is a child that calls `setsid` to
daemonize. cgroups close that gap, but they need cgroup v2, a writable
(delegated) cgroup directory, and a way to start the child inside it.
`std::process::Command` exposes none of that, so it takes a `pre_exec`
write to `cgroup.procs` and a runtime fallback. Job Objects need a
`windows-sys` dependency, and Bear has too few Windows users to justify
that path.

The group is made with the safe `Command::process_group(0)`. The
compiler version probe uses `setsid` instead. The two are not
interchangeable: the probe needs its own session, and the reason is in
a comment next to the probe code.

Waiting was the second problem. `Child::wait()` blocks and cannot watch
for a signal at the same time, so the old loop polled with `try_wait()`
and a 100ms sleep. Options:

- Keep polling. Rejected: it adds up to 100ms latency to every signal.
- A `poll()` over a `pidfd` and a `signalfd`. Rejected: nicer, but
  Linux 5.3+ only and more `libc` code.
- A blocking loop on `SIGCHLD` plus the termination signals, using the
  `signal-hook` crate Bear already has. Chosen: portable and no poll.

Nesting was the third. In wrapper mode the chain is `bear-driver` ->
`make` -> `bear-wrapper` -> real `cc`, and the wrapper runs the same
`supervise()`. If every level made a new process group, the build would
split into many groups. A top-level `killpg` would then miss the deeper
processes, which reopens the hole grouping is meant to close.

## Decision

The outermost supervisor puts the build in its own process group and,
on Linux, also in a fresh cgroup. On a signal it forwards the real
signal to the group, waits a short grace window, then sends `SIGKILL` to
the group, or kills the cgroup when there is one. Nested supervisors
stay in the inherited group and only relay the signal to their child.

## Consequences

- No new dependency. The wait reacts at signal speed, inside the budget.
- Build tools such as `make -j`, `ninja` and `cmake --build` stop their
  own workers when signaled. Forwarding the real signal keeps that
  working, so Bear does not reimplement worker teardown per tool. This
  is why the requirement says Bear must not interfere with the build
  tool's own signal handling.
- The graceful phase is group-based because `cgroup.kill` can only send
  `SIGKILL`. A descendant that detached with `setsid` gets no grace
  window, only the final `cgroup.kill`. Accepted: a daemon that detaches
  on purpose gives up the graceful wind-down.
- Without a usable cgroup (not Linux, no cgroup v2, no delegated
  directory, kernel without `cgroup.kill`) the `setsid` hole stays open
  and teardown falls back to the process group.
- The build leaves Bear's process group, so the terminal no longer sends
  Ctrl-C to the build directly. Bear is the only path for the signal.
  This is what makes tree teardown and real-signal forwarding reliable,
  and it fixes traps in the non-terminal case (CI `SIGTERM`). The cost is
  that a gap in Bear's forwarding has no terminal backstop. Accepted.
- Each build creates and removes one cgroup directory. Killed processes
  are reaped by init later, so removal after a kill retries briefly.
- The wrapper stays simple (forward and pass the exit code back). It
  inherits the leader's cgroup through its parent, so one `cgroup.kill`
  reaches the whole tree.
- Outside unix, Bear still kills only the direct child. Revisit with a
  Job Object if Windows users report builds that outlive Bear.

## References

- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the requirement this supports
- [#713](https://github.com/rizsotto/Bear/issues/713), the starved CI runner that sized the grace window

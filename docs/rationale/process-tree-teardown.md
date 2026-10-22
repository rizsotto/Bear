# Supervise the build

## Context

When Bear runs a build (`bear -- make`), `bear-driver` starts the build
and supervises it as the leader. It waits for the build to end and
passes its exit status back. When a termination signal arrives, the
whole process tree under the build must stop within the budget that
[cli-signal-forwarding](../requirements/cli-signal-forwarding.md) sets.
Bear should still write the partial `compile_commands.json` it has
collected. The first `supervise()` only sent `SIGKILL` to the direct
child. Grandchildren were reparented to init and kept running. And
`SIGKILL` cannot be trapped, so neither the build's own trap nor running
compilers got a chance to wind down.

The leader's job has four steps.

**Start.** The leader puts the build in its own process group. On Linux
it also puts the build in a fresh cgroup. Three mechanisms can stop a
whole tree:

| Platform | Mechanism | Can a child escape? |
|---|---|---|
| any unix | process group (`setpgid`/`setsid`) + `killpg` | yes, by calling `setsid` itself |
| Linux | cgroup v2 `cgroup.kill` | no, unprivileged moves are denied |
| Windows | Job Object (`KILL_ON_JOB_CLOSE`) | no |

Process groups work on every unix and need no new dependency (`libc` is
already there). Their one gap is a child that calls `setsid` to
daemonize. cgroups close that gap, but they need cgroup v2, a writable
(delegated) cgroup directory, and a kernel with `cgroup.kill`.
`std::process::Command` cannot start a child in a cgroup, so the child
writes its own pid to `cgroup.procs` in a `pre_exec` hook. When any of
this is missing, Bear falls back to the process group at runtime. Job
Objects need a `windows-sys` dependency, and Bear has too few Windows
users to justify that path.

The group is made with the safe `Command::process_group(0)`. The
compiler version probe uses `setsid` instead, for a reason given in a
comment next to the probe code.

**Wait.** The leader blocks until the build exits or a signal arrives.
`Child::wait()` blocks and cannot watch for a signal at the same time.
Options:

- Poll with `try_wait()` and a sleep, as the old loop did with 100ms.
  Rejected: every signal can wait up to one full sleep.
- A `poll()` over a `pidfd` and a `signalfd`. Rejected: Linux 5.3+ only
  and more `libc` code.
- A blocking loop on `SIGCHLD` plus the termination signals, using the
  `signal-hook` crate Bear already has. Chosen: portable and no polling.

On unix the termination signals are `SIGTERM`, `SIGINT` and `SIGQUIT`.
Windows has no `SIGCHLD`, so there Bear polls every 100ms.

**Forward.** The leader sends the real signal it got to the whole
process group, not a substitute. Build tools such as `make -j`, `ninja`
and `cmake --build` stop their own workers when signaled, and the
build's traps can tell `SIGINT` from `SIGTERM`.

**Grace and kill.** The leader then gives the tree a grace window of
800ms to wind down. [#713](https://github.com/rizsotto/Bear/issues/713)
sized it: a trap on a starved CI runner needed the headroom, and 800ms
still keeps a build that ignores the signal inside the budget. During
the window the leader checks the direct child with `try_wait()` every
10ms. If the child exits, the window ends early. Then comes the kill.
With a cgroup, the leader always writes `cgroup.kill`, even when the
child exited in time, to reach a descendant that left the group. Without
a cgroup, it sends `SIGKILL` to the group only if the direct child is
still there.

A `pidfd` for the grace window was drafted and dropped. It was a
Linux-only path with a runtime fallback for `ENOSYS` and `EPERM`, about
a hundred extra lines and three `unsafe` blocks. The gain was at most 10ms and
one wakeup instead of about 80, and only after a signal. Next to an
800ms grace window nobody would notice.

The wrapper mechanism runs the same `supervise()` deeper in the tree.
That part is in [supervise-a-compiler-call](supervise-a-compiler-call.md).

## Decision

The leader starts the build in its own process group and, on Linux, in
a fresh cgroup, then waits on `SIGCHLD` and the termination signals. On
a signal it forwards the real signal to the group and gives it 800ms.
Then it kills the cgroup, or sends `SIGKILL` to the group if the child
is still there.

## Consequences

- No new dependency. The wait reacts at signal speed, inside the budget.
- Forwarding the real signal keeps the build tools' own worker cleanup
  working, so Bear does not reimplement it per tool. This is why the
  requirement says Bear must not interfere with the build tool's own
  signal handling.
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
- Outside unix, Bear still kills only the direct child. Revisit with a
  Job Object if Windows users report builds that outlive Bear.

## References

- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the requirement this supports
- [supervise-a-compiler-call](supervise-a-compiler-call.md), the nested supervisor under the wrapper mechanism
- [#713](https://github.com/rizsotto/Bear/issues/713), the starved CI runner that sized the grace window

# Process-tree teardown and event-driven supervision

## Context

When Bear supervises a build (`bear -- make`) and a termination signal
arrives, the whole process subtree underneath the build must stop, within
the sub-one-second budget the signal-forwarding requirement sets, and Bear
should still be able to write the partial `compile_commands.json` it has
collected so far. The original `supervise()` only `child.kill()`ed the
direct child with `SIGKILL`, which (a) left grandchildren reparented to
init and running, and (b) being un-trappable, gave neither a build's own
trap nor in-flight compilers any chance to wind down.

Three mechanisms can terminate an entire subtree, one per platform family:

| Platform | Mechanism | A child can escape it? |
|---|---|---|
| any unix | process group (`setsid`/`setpgid`) + `killpg` | yes - by calling `setsid` itself |
| Linux | cgroup v2 `cgroup.kill` | no - unprivileged moves are denied |
| Windows | Job Object (`KILL_ON_JOB_CLOSE`) | no |

Process groups are portable across unix and need no new dependency (`libc`
is already present); the same group-kill *technique* is already proven in
the version-probe watchdog (`semantic/interpreters/compilers/probe.rs`),
though Stage 1 uses the lighter `Command::process_group(0)` (safe std, keeps
the session) rather than the watchdog's `unsafe` `setsid`. Their one gap is
a child that
deliberately `setsid`s away to daemonize. cgroups close that gap but require
cgroup v2, a writable/delegated cgroup directory, and
`clone3(CLONE_INTO_CGROUP)` or a `pre_exec` write to `cgroup.procs` - none
exposed by `std::process::Command` - plus a runtime fallback. Job Objects
need a `windows-sys` dependency, and Bear has too few Windows users to
justify designing that path yet.

Two further forces shaped the design:

- **Waiting without polling.** `std::process::Child::wait()` blocks
  uninterruptibly and cannot watch for a signal at the same time, which is
  why the original loop polled with `try_wait()` + `sleep(100ms)`. A
  SIGCHLD-driven blocking loop (portable, reuses the already-present
  `signal-hook`) removes the poll and its latency; a Linux-only `poll()`
  over a `pidfd` + `signalfd` would be strictly nicer but Linux-5.3+ and
  more `libc` code.

- **Nested supervisors.** In wrapper mode the chain is `bear-driver` ->
  `make` -> `bear-wrapper` -> real `cc` (the wrapper is a Rust binary on the
  same `supervise()` path, not a shell script). If *every* level created a
  new process group, the build would fragment into many groups and a
  top-level `killpg` would miss the deeper processes - re-opening the very
  escape hole grouping is meant to close.

## Decision

- **Two-stage tree teardown.** Stage 1 puts each supervised build in its
  own process group and signals the group -- portable across unix and
  needing no new dependency. Stage 2, on Linux only, additionally places
  the build in a fresh cgroup v2 and reaps it with `cgroup.kill`, catching
  a descendant that `setsid`s out of the process group; it is best-effort
  and falls back to Stage 1 when cgroup v2 is unavailable or its directory
  is not delegated. Both go through the leader's single grace-then-force
  escalation; the graceful phase stays group-based because `cgroup.kill`
  can only `SIGKILL`. A Windows Job Object is a possible later third path;
  non-unix keeps single-process kill.
- **Only the outermost supervisor groups.** The driver creates the group
  and owns the authoritative group-kill; nested wrappers inherit the group
  and merely forward, so a single top-level kill reaches the whole tree.
  Grouping is a per-caller policy, not baked unconditionally into shared
  supervision.
- **Graceful, real-signal forwarding.** Forward the signal Bear actually
  received (not a hardcoded one) to the group, give the tree a grace window
  to wind down and let Bear write the partial database, then escalate to
  `SIGKILL`.
- **Event-driven wait** replaces the poll: a signal-driven blocking wait
  reacts at signal speed, and the grace-then-`SIGKILL` escalation runs off
  a deadline inside that loop.

## Consequences

- No new dependency for Stage 1: the primitives it needs are already in the
  tree, and the group-kill technique is borrowed from the existing watchdog.
- The poll and its up-to-100ms latency are gone; teardown reacts at signal
  speed, inside the budget.
- The child leaves Bear's process group, so the tty no longer delivers
  Ctrl-C to the build directly - Bear becomes the sole conduit. This is what
  makes reliable tree-kill and real-signal forwarding possible and fixes
  trap support in the non-tty (CI `SIGTERM`) case; the trade-off is that any
  gap in Bear's forwarding loses the tty backstop. Accepted.
- Stage 2 closes the `setsid`-escape hole on Linux hosts with a usable
  cgroup; where none is available the hole remains and the documented
  process-group fallback applies. A descendant that detaches gets no grace
  window (it left the group the graceful signal targets) - only the final
  `cgroup.kill`. Accepted: a daemon that deliberately detaches forfeits the
  graceful wind-down.
- Each supervised build creates and removes one cgroup directory; a normal
  build leaves nothing behind, and the kill path's directory cleanup retries
  briefly because killed processes are reaped asynchronously by init.
- The "only the outermost supervisor groups" rule keeps wrapper-mode nesting
  correct and keeps the wrapper's supervision simple (forward + propagate
  exit code); the wrapper inherits the leader's cgroup through the child, so
  one `cgroup.kill` reaches the whole tree.
- The cgroup stays a Linux-only, runtime-detected layer; a Windows Job
  Object and a pidfd-based wait remain possible later additions.

## Rejected: unifying the probe watchdog on `process_group(0)`

It is tempting to drop the `unsafe setsid` in the version-probe watchdog
(`semantic/interpreters/compilers/probe.rs`) and reuse Stage 1's safe
`Command::process_group(0)`, on the theory that `setsid`'s extra
controlling-terminal detach is a no-op once `stdin` is null and
`stdout`/`stderr` are pipes. Testing refutes it: under the parallel probe
suite, `process_group(0)` produced intermittent misclassification (3 of 8
runs failed) while `setsid` did not (0 of 13). `setsid` gives each
short-lived probe its own session with no controlling terminal;
`process_group(0)` leaves it a background group inside the test runner's
session and terminal, and under concurrency that difference is observable.
The two calls are therefore *not* interchangeable in general - the lighter
one is right for a single supervised build (Stage 1) but wrong for the
probe. The probe keeps `setsid`.

## References

- Requirement: `cli-signal-forwarding`
- Prior art in-tree: the version-probe watchdog's `setsid` + `killpg`
  teardown (`semantic/interpreters/compilers/probe.rs`)
- Plan: `plan.md` (repo root, transient)

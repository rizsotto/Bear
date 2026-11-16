# Do not supervise the build

## Context

When Bear runs a build (`bear -- make`), `bear-driver` starts the build
and waits for it. Earlier, Bear acted as the build's supervisor. It put
the build in its own process group and, on Linux, in a cgroup. On a
termination signal it forwarded the signal to the group, waited 800ms,
then killed the group or the cgroup. The goal was to stop the whole tree
within a time budget.

That design broke what the terminal does for a build. Job control works
per process group. With the build in its own group, it could not read the
terminal, Ctrl-Z did not stop it, and a closed terminal did not reach it.
No amount of forwarding fixes those, because they are not signals Bear
can forward. The grace window made the build behave differently than
without Bear, and it caused
[#713](https://github.com/rizsotto/Bear/issues/713), where a trap on a
starved CI runner needed more time. Its tests needed retry loops.

The goal now is that a build runs the same with and without Bear. Bear
stands in for the build. It does not supervise it.

Options for the build's place:

- Keep the own group and forward, as before. Rejected: it cannot fix
  terminal input or Ctrl-Z.
- Keep the own group and hand the terminal to it (`tcsetpgrp`, watch the
  child for stops, stop Bear, take the terminal back on `fg`). Rejected:
  it is a job-control shell inside Bear. Much `unsafe` code, many corner
  cases (Bear in the background, no terminal, nested shells), and still
  not the same as no Bear.
- Choose at runtime: own group only when there is no terminal. Rejected:
  two behaviors are twice the tests and half the predictability.

Options for the relay:

- Relay to the whole process group. Rejected: from a script Bear shares
  its group with the script and maybe the CI runner. A group kill from
  Bear would hit its own parents.
- Relay to the group only when Bear is the group leader. Rejected: it
  works more often, but is wrong in a pipeline (`bear -- make | tee log`
  would signal `tee`).
- Never relay `SIGINT` and `SIGQUIT`. Rejected: it breaks the caller
  that sends `SIGINT` to the pid it started.
- Always relay. Rejected: a terminal Ctrl-C then reaches the top process
  of the build twice. A tool that counts presses, where a second Ctrl-C
  means "force quit", would be affected.
- Skip when the child's group is the terminal's foreground group.
  Rejected: it drops real signals, such as `kill -INT <bear pid>` from
  another terminal.
- Skip when the sender is the child or in the child's group (the rule of
  `sudo`). Rejected: it drops the `SIGTERM` that make sends to its jobs.
- Skip when there is no sender pid, or pid 0. Rejected: a `kill` from
  outside a PID namespace (`docker stop`) arrives with pid 0, and Bear
  would wait forever.

Options for the end of the wait:

- Keep the grace window and the kill. Rejected: see above.
- A longer or configurable grace window. Rejected: still a behavior the
  build does not have without Bear, and a new flag.
- Keep the cgroup only to sweep leftovers. Rejected: it is Linux only,
  it leaks a directory when a daemon survives, and killing a daemon the
  build started on purpose is not Bear's call.

## Decision

The build stays in Bear's process group and session. It is a plain child.
Bear relays every termination signal to its direct child and to nothing
else. It skips a relay only when all three hold: the signal is `SIGINT`
or `SIGQUIT`, the signal came from the kernel (a terminal key), and the
child is still in Bear's process group. In that case the terminal
already reached the child. Bear sends no signal of its own, and waits as
long as the build takes.

Why the proof of the kernel as the sender is needed: a kill from outside
a PID namespace and, on macOS, every signal, carry no usable sender. An
unknown sender must relay, because a missed signal leaves the build
running and a double one at worst runs a trap twice.

`SIGHUP` and `SIGTERM` are always relayed. The terminal never sends
`SIGTERM`. On a hang-up the kernel signals the session leader only, and
the rest of the group gets it later. When Bear is the leader, skipping
would leave Bear waiting for a child that never gets the signal.

A signal that was ignored when Bear started stays ignored, and `SIGHUP`
is watched. Ignored signals survive `exec`. A handler does not. If Bear
registered a handler for an ignored signal, it would un-ignore that
signal for the build, as in `bear -- make &` from a script.

On Windows the console gives Ctrl-C to every attached process, so Bear
relays nothing. It notes the interrupt, waits, cleans up, and ends like
the build.

## Consequences

- Terminal input, Ctrl-Z, terminal close, and signals sent to the process
  group work with no code, because the kernel does them.
- Bear no longer stops the whole tree, and no longer stops a build that
  ignores a signal. Nobody does that without Bear either.
- The relay decision is one pure function that a table test covers with
  no processes. The `GroupPolicy` trait, its two implementations, and the
  generic parameter go. A trait whose second implementation is a test
  double does not earn its place, and two names for one behavior invite
  someone to make them differ again.
- A signal can reach the top process of the build twice: when a process
  signals the whole group, and on macOS and BSD, where no signal is
  proven to come from the kernel. Accepted. It is the safe way to fail.
- The sender is known through signal-hook's `extended-siginfo` feature,
  which needs a small C file built with the `cc` crate. The workspace
  already needs a C compiler. Revisit the macOS rule if a reliable marker
  for terminal signals shows up there.
- A build tool that moves itself to its own process group (`timeout`)
  ends up in the background under Bear. Ctrl-C still reaches it through
  the relay. Bear cannot `exec` the build, so this stays.

## References

- [cli-signal-forwarding](../requirements/cli-signal-forwarding.md), the requirement this supports
- [supervise-a-compiler-call](supervise-a-compiler-call.md), the wrapper under the same rules
- [end-like-the-build](end-like-the-build.md), how Bear ends after the build
- [abort-on-signal](abort-on-signal.md), a signal when no build runs
- [#713](https://github.com/rizsotto/Bear/issues/713), the starved CI runner that the old grace window hit

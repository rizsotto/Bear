---
title: Signal forwarding to the build
status: implemented
---

## Intent

When the user runs `bear -- make` in a terminal and presses `SIGINT`
(Ctrl-C), the interrupt must reach the build. Bear must not swallow
the signal, leaving `make` and the whole process tree underneath it
running in the background. The same applies when a CI runner sends
`SIGTERM` to a running Bear to abort a build: the build being
supervised must stop too.

The exit code a signalled build produces -- and Bear's exit-code
contract in general -- is owned by
[`cli-exit-codes`](cli-exit-codes.md); this requirement covers only how
the signal reaches the build and how the process tree is torn down.

Users interact with a single command, `bear`. Which interception mode
runs is owned by
[`interception-mode-selection`](interception-mode-selection.md); the
contract below must hold identically in every mode.

## Acceptance criteria

- Pressing Ctrl-C while `bear -- <build>` runs stops the build
- Sending `SIGTERM` (and `SIGQUIT` on Unix) to a running Bear stops
  the build
- Between the signal arriving and both Bear and the build ending,
  less than one second elapses on a system not under heavy load
- The build receives the same signal Bear received: a `SIGINT` arrives
  as `SIGINT` and a `SIGTERM` as `SIGTERM`, so a build that handles the
  two differently sees the real one. Bear relays the signal rather than
  substituting one of its own
- The whole process tree under the build is stopped, not just the
  immediate child: descendants the build spawned do not survive Bear
- If the build is a shell script that installs its own signal trap,
  the script receives the signal, its trap runs, and Bear's exit
  code reflects whatever the script ultimately exited with
- The build is first asked to stop with the received signal and given
  a brief grace period to shut down cleanly before it is forced; a
  build that ignores the signal is still stopped within the time budget
- Every acceptance criterion above applies to every supported
  interception mode

## Non-functional constraints

- Platform support: Linux, macOS, BSD, and Windows
- Bear must not interfere with the build tool's own signal handling
- Running a build under `bear --` must add no perceptible delay
  compared with running the same command directly

## Known limitations

**A child that re-detaches into its own session can still survive.**
Bear stops the whole process tree by signalling the build's process
group. A child that deliberately starts a new session of its own (a
daemon that calls `setsid`) leaves that group and may keep running
after Bear exits. On Linux, where the host provides a usable cgroup,
Bear closes this gap by terminating the build's cgroup instead, which a
child cannot leave; where no such cgroup is available Bear falls back
to the process-group behaviour and the limitation applies.

**Windows signal coverage is limited.** Only `SIGTERM` and `SIGINT`
are observed. Other Windows-specific termination mechanisms (such as
`CTRL_BREAK_EVENT` or a parent calling `TerminateProcess` on
`bear.exe`) are not explicitly handled.

**Nested `bear` invocations are unsupported.** Running
`bear -- bear -- make` is not a supported configuration and its
signal/exit-code behaviour is undefined. Users who need to record
multiple separate builds into a single database should use
`--append` (see `output-append`) rather than nesting Bear.

## Testing

Given a long-running build under `bear --`:

> When the test runs `bear -- sleep 10`, waits briefly for the sleep
> to start, then sends a termination signal to the `bear` process,
> then both `bear` and the `sleep` child terminate within one
> second,
> and `bear` reports a non-success exit status.

Given a build interrupted mid-compile -- the compiler's input source is
a named pipe created by `mkfifo` with no writer, so the compiler blocks
reading it:

> When the test runs `bear -- cc -x c -c source.c -o out.o` with
> `source.c` the pipe, waits for the compiler to block, then sends a
> termination signal to the `bear` process (standing in for the
> interactive Ctrl-C),
> then both `bear` and the compiler terminate within the time budget.

Given a build script that installs distinct traps for `SIGINT` and
`SIGTERM` -- each trap writes its own marker file and exits with its own
code (`10` for `INT`, `20` for `TERM`) -- then blocks in a long sleep:

> When the test runs the script under `bear --`, waits until the script
> reports it is ready, and sends `SIGINT` to the `bear` process,
> then the `INT` marker appears (the `TERM` trap did not fire)
> and `bear` exits `10`;
> and when the test repeats the run sending `SIGTERM` instead,
> then the `TERM` marker appears and `bear` exits `20` -- the build saw
> the real signal, not a substitute.

Given a build script that spawns a background grandchild (`sleep 60 &`),
records the grandchild's pid, then blocks in its own `sleep 60`:

> When the test sends `SIGTERM` to the `bear` process,
> then `bear` exits with a non-success status,
> and the recorded grandchild pid is no longer alive -- teardown reached
> the whole tree, not just the direct child.
> (On Linux with a usable cgroup the same holds even when the grandchild
> starts its own session with `setsid`; without one, the process-group
> fallback in Known limitations applies.)

Given a build script that traps `SIGTERM`, writes a marker file from the
trap, and exits `42` from it:

> When the test sends `SIGTERM` to the `bear` process while the script
> blocks in a long sleep,
> then the marker appears -- the script received the signal and its trap
> ran, rather than being killed outright --
> and `bear` exits `42`, the script's own exit code.

Given a build script that ignores `SIGTERM` (an empty trap) and loops
forever:

> When the test sends `SIGTERM` to the `bear` process,
> then both `bear` and the build still end within the time budget -- the
> grace period expires and the build is forced --
> and `bear` exits with a non-success status.

Given wrapper mode selected by configuration, and a build that invokes
the compiler through `$CC -c` on a named pipe with no writer, so the
compiler blocks mid-compile under the wrapper's supervision:

> When the test sends `SIGTERM` to the `bear` process,
> then both `bear` and the blocked compiler end within the time budget,
> and `bear` exits with a non-success status -- the teardown contract
> holds in wrapper mode just as in preload mode.

## Rationale

- [process-tree teardown and graceful forwarding](../rationale/process-tree-teardown.md)

---
title: Signal forwarding to the build
status: implemented
---

## Intent

When the user runs a build under Bear in a terminal and presses Ctrl-C
(`SIGINT`), the interrupt must reach the build. Bear must not swallow it
and leave `make` and the whole process tree under it running in the
background. The same holds when a CI runner sends `SIGTERM` to Bear to
abort a build: the build must stop too.

This file covers how the signal reaches the build and how the process
tree is torn down. Bear's exit code, after a signal included, is owned by
[`cli-exit-codes`](cli-exit-codes.md). The contract holds the same with
the preload mechanism and with the wrapper mechanism; which one runs is
owned by
[`interception-mechanism-selection`](interception-mechanism-selection.md).

## Acceptance criteria

- **Given** a build running under Bear (`sleep 10`), **when** the user
  presses Ctrl-C, or `SIGTERM` (or `SIGQUIT` on Unix) is sent to Bear,
  after the sleep starts, **then** the build stops, Bear and the sleep
  both end in less than one second on a system not under heavy load (the
  time budget), and Bear exits with a non-success status.
- **Given** a compiler blocked mid-compile (its source is a named pipe
  with no writer), **when** a termination signal is sent to Bear, standing
  in for Ctrl-C, **then** Bear and the compiler both end within the time
  budget.
- **Given** a build script with distinct traps for `SIGINT` and `SIGTERM`,
  each writing its own marker file and exiting with its own status,
  **when** `SIGINT` is sent to Bear, **then** the build gets `SIGINT`, not
  a substitute: only the `INT` marker appears and the `INT` trap's exit
  status passes through (as `cli-exit-codes` states). **When** `SIGTERM`
  is sent instead, **then** only the `TERM` marker appears and the `TERM`
  trap's exit status passes through.
- **Given** a build script that traps `SIGTERM`, writes a marker from the
  trap, and exits with its own status, **when** `SIGTERM` is sent to Bear
  while the script sleeps, **then** the build is first asked to stop with
  `SIGTERM` and gets a short grace period: the marker appears, so the
  trap ran rather than the script being killed, and the trap's exit
  status passes through (as `cli-exit-codes` states).
- **Given** a build script that starts a background grandchild
  (`sleep 60 &`) and then sleeps itself, **when** `SIGTERM` is sent to
  Bear, **then** Bear exits with a non-success status and the grandchild
  is gone. The whole tree is stopped, not just the direct child. On Linux
  with a usable cgroup this holds even when the grandchild starts its own
  session with `setsid`.
- **Given** a build script that ignores `SIGTERM` and loops forever,
  **when** `SIGTERM` is sent to Bear, **then** the grace period runs out,
  the build is forced to stop, both end within the time budget, and Bear
  exits with a non-success status.
- **Given** the wrapper mechanism and a build that runs `$CC -c` on a
  named pipe with no writer, so the compiler blocks under the wrapper,
  **when** `SIGTERM` is sent to Bear, **then** Bear and the blocked
  compiler both end within the time budget, and Bear exits with a
  non-success status.

## Non-functional constraints

- Platform support: Linux, macOS, BSD, and Windows.
- Bear does not interfere with the build tool's own signal handling.
- Running a build under Bear adds no noticeable delay compared with
  running the same command directly.

## Known limitations

- A child that starts a new session of its own (a daemon that calls
  `setsid`) leaves the build's process group and may outlive Bear. On
  Linux with a usable cgroup, Bear stops the build's cgroup instead,
  which a child cannot leave. Without one, the limitation applies.
- On Windows only `SIGTERM` and `SIGINT` are observed. Other ways to end
  a process (`CTRL_BREAK_EVENT`, a parent calling `TerminateProcess` on
  Bear) are not handled.
- Nested Bear runs (Bear running under another Bear) are not supported,
  and their signal and exit-code behavior is undefined. To record several
  builds into one database, run them one after the other while appending
  ([`output-append`](output-append.md)).

## Rationale

- [process-tree teardown and graceful forwarding](../rationale/process-tree-teardown.md)

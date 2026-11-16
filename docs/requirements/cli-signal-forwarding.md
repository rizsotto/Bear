---
title: Signals and the build
status: accepted
---

## Intent

Bear adds nothing and removes nothing. A signal reaches the build as it
would without Bear, and Bear reports the build's end as its own. This
holds in every mode that runs a build, and for the wrapper.

While a build runs, a termination signal belongs to the build. Bear
passes it on to the process it started, unless the terminal already
gave it to that process, and sends no signal of its own.
Bear waits for the build as long as the build takes. While no build
runs, a termination signal aborts Bear.

Termination signals are `SIGHUP`, `SIGINT`, `SIGQUIT`, and `SIGTERM` on
Unix, and Ctrl-C and `SIGTERM` on Windows.

This file covers how the signal reaches the build and what Bear does
with it. Bear's exit code, after a signal included, is owned by
[`cli-exit-codes`](cli-exit-codes.md). What happens to the output is
owned by [`output-atomic-write`](output-atomic-write.md) and
[`output-append`](output-append.md). The contract holds the same with
the preload mechanism and with the wrapper mechanism; which one runs is
owned by
[`interception-mechanism-selection`](interception-mechanism-selection.md).

## Acceptance criteria

- **Given** a build running under Bear (`sleep 10`), **when** `SIGTERM`
  (or `SIGINT`, `SIGQUIT`, `SIGHUP` on Unix) is sent to Bear after the
  sleep starts, **then** the build stops, and Bear and the sleep both
  end in less than one second on a system not under heavy load (the time
  budget). How Bear ends is owned by `cli-exit-codes`.
- **Given** a compiler blocked mid-compile (its source is a named pipe
  with no writer), **when** a termination signal is sent to the process
  group of Bear, standing in for Ctrl-C, **then** Bear and the compiler
  both end within the time budget.
- **Given** a build script with distinct traps for `SIGINT` and `SIGTERM`,
  each writing its own marker file and exiting with its own status,
  **when** `SIGINT` is sent to Bear, **then** the build gets `SIGINT`, not
  a substitute: only the `INT` marker appears and the `INT` trap's exit
  status passes through (as `cli-exit-codes` states). **When** `SIGTERM`
  is sent instead, **then** only the `TERM` marker appears and the `TERM`
  trap's exit status passes through.
- **Given** a build script that traps `SIGTERM`, writes a marker from the
  trap after a delay, and exits with its own status, **when** `SIGTERM`
  is sent to Bear while the script sleeps, **then** the trap runs to the
  end, whatever the delay: the marker appears, Bear sends no other
  signal, and the trap's exit status passes through (as `cli-exit-codes`
  states).
- **Given** a build script that starts a background grandchild
  (`sleep 60 &`) and then sleeps itself, **when** `SIGTERM` is sent to
  the process group of Bear, **then** the grandchild is gone. **When**
  `SIGTERM` is sent to Bear alone, **then** the grandchild is still
  running, as it would be without Bear.
- **Given** a build script that ignores `SIGTERM` and loops forever,
  **when** `SIGTERM` is sent to Bear, **then** Bear does not stop the
  build and keeps waiting for it.
- **Given** the wrapper mechanism and a build that runs `$CC -c` on a
  named pipe with no writer, so the compiler blocks under the wrapper,
  **when** `SIGTERM` is sent to the process group of Bear, **then** Bear
  and the blocked compiler both end within the time budget.
- **Given** a build run under Bear on Linux, **when** the user presses
  Ctrl-C or Ctrl-\ in the terminal, **then** the top process of the build
  gets the signal once.
- **Given** a signal that is ignored when Bear starts (as in a script
  that starts Bear in the background), **when** the build runs, **then**
  the build starts with that signal ignored, and a signal sent to Bear
  does not stop Bear or the build.
- **Given** a build running under Bear, **when** its terminal is closed
  (`SIGHUP`), **then** the build gets `SIGHUP`. Bear's end and its temporary
  artifacts are owned by `cli-exit-codes` and `output-atomic-write`.
- **Given** a Bear run with no build running (semantic mode, parse-sh
  mode, or the end of a build run while Bear writes the output),
  **when** a termination signal is sent to Bear, **then** Bear aborts.
  How it ends is owned by `cli-exit-codes`, and what it leaves behind
  by `output-atomic-write`.
- **Given** a build under Bear in a terminal, **when** the user presses
  Ctrl-Z and then continues the job, **then** Bear and the build stop
  together and continue together.
- **Given** a build under Bear in a terminal, **when** the build reads
  from the terminal, **then** it gets the user's input as it would
  without Bear.
- **Given** a build under Bear, **when** it runs, **then** the build is
  in the process group and the session of Bear, as it would be without
  Bear.
- **Given** the wrapper and a compiler call, **when** a termination
  signal reaches the wrapper, **then** the wrapper follows the same
  rules as Bear: it passes the signal on to the compiler, sends no
  signal of its own, and waits for the compiler.
- **Given** Windows, **when** Ctrl-C reaches Bear while a build runs,
  **then** Bear waits for the build, cleans up, and ends with the
  build's exit code.

## Non-functional constraints

- Platform support: Linux, macOS, BSD, and Windows.
- Bear does not interfere with the build tool's own signal handling.
- Running a build under Bear adds no noticeable delay compared with
  running the same command directly.

## Known limitations

- Bear cannot catch `SIGKILL` and `SIGSTOP`. When one is sent to Bear,
  the build runs on. What Bear leaves behind is owned by
  `output-atomic-write`.
- Bear does not stop the whole tree and does not force a build to stop.
  A background grandchild or a daemon survives a signal sent to Bear
  alone, and a build that ignores signals keeps Bear waiting. Both are
  how it is without Bear.
- A signal can reach the top process of the build twice. When a process
  signals the whole process group, the build gets it from the sender and
  again from Bear. On a hang-up, when Bear is not the session leader,
  the build gets `SIGHUP` from the kernel and from Bear.
- On macOS and BSD, Ctrl-C and Ctrl-\ in a terminal reach the top
  process of the build twice.
- A build tool that moves itself to a process group of its own (`timeout`
  without its foreground option) runs in the background under Bear and
  cannot read the terminal. Ctrl-C still reaches it.
- The build always starts with `SIGPIPE` at its default action, and
  with `SIGCHLD` at its default action, even when Bear started with
  either one ignored.
- A signal that arrives between the start of the build process and the
  moment it runs the build program is lost for the build.
- On Windows only Ctrl-C and `SIGTERM` are observed. Other ways to end
  a process (`CTRL_BREAK_EVENT`, closing the console, a parent calling
  `TerminateProcess` on Bear) are not handled.
- Nested Bear runs (Bear running under another Bear) are not supported,
  and their signal and exit-code behavior is undefined. To record several
  builds into one database, run them one after the other while appending
  ([`output-append`](output-append.md)).

## Rationale

- [Do not supervise the build](../rationale/process-tree-teardown.md)
- [The wrapper does what Bear does](../rationale/supervise-a-compiler-call.md)
- [End like the build](../rationale/end-like-the-build.md)
- [Abort on a signal when no build runs](../rationale/abort-on-signal.md)

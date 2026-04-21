---
title: Signal forwarding and exit-code propagation
status: implemented
---

## Intent

When the user runs `bear -- make` in a terminal and presses `SIGINT`
(Ctrl-C), the interrupt must reach the build. Bear must not swallow
the signal, leaving `make` and the whole process tree underneath it
running in the background. The same applies when a CI runner sends
`SIGTERM` to a running Bear to abort a build: the build being
supervised must stop too.

The reverse direction matters just as much. Shells, CI runners, and
parent `make` rules look at Bear's exit code to decide whether the
build succeeded. Bear must therefore report the same exit code that
the build command produced, so the caller sees the real result rather
than a success or failure Bear invented.

Users interact with a single command, `bear`. The interception mode
Bear selects (see `interception-preload-mechanism` and
`interception-wrapper-mechanism`) is an internal detail: the contract
below must hold identically regardless of which mode is active.

## Acceptance criteria

- Pressing Ctrl-C while `bear -- <build>` runs stops the build
- Sending `SIGTERM` (and `SIGQUIT` on Unix) to a running Bear stops
  the build
- Between the signal arriving and both Bear and the build ending,
  less than one second elapses on a system not under heavy load
- When the build exits normally, Bear's exit code equals the build
  command's exit code: `0` is preserved for success, non-zero codes
  are preserved for failure
- Exit codes in the portable range (0-255 on Unix) are propagated
  byte-for-byte
- When the build is terminated by a signal rather than exiting
  normally, Bear exits with a non-zero code so that scripts and CI
  systems see a failed build
- If the build is a shell script that installs its own signal trap,
  the script receives the signal, its trap runs, and Bear's exit
  code reflects whatever the script ultimately exited with
- Every acceptance criterion above applies to every supported
  interception mode

## Non-functional constraints

- Platform support: Linux, macOS, BSD, and Windows
- Bear must not interfere with the build tool's own signal handling.
  Build drivers such as `make -j`, `ninja`, and `cmake --build`
  install their own handlers to stop their workers on termination;
  Bear relies on that behaviour rather than re-implementing it
- Running a build under `bear --` must add no perceptible delay
  compared with running the same command directly

## Known limitations

**The specific signal is not distinguished.** `SIGINT` and `SIGTERM`
both cause Bear to tear down the build immediately. A caller that
wanted to use `SIGTERM` for "graceful stop" and `SIGINT` for
"interactive interrupt" will observe the same behaviour for both.

**The signal that terminated the build is not encoded in Bear's exit
code.** The shell convention of `128 + signal_number` is not
followed. Scripts that inspect Bear's exit code to identify *why* a
build stopped cannot distinguish signal termination from a regular
build failure.

**Daemon-style grandchildren may survive.** If the build spawns
processes that detach from their parent (for example, a background
service started by the build), they may keep running after Bear has
exited. Well-behaved build drivers (`make`, `ninja`) avoid this by
propagating termination downwards themselves.

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

Protected by `exit_code_when_signaled` (gated on the presence of a
`sleep` executable on the host).

Given a build that exits successfully:

> When the user runs `bear -- true`,
> then `bear` exits with code `0`.

Protected by `exit_code_for_true`.

Given a build that exits with a non-zero code:

> When the user runs `bear -- false`,
> then `bear` exits with a non-zero code matching the build's.

Protected by `exit_code_for_false`.

Given an interception-only run (`bear intercept`), the exit-code
contract still holds:

> When the user runs `bear intercept -- true`, the exit code is `0`;
> when the user runs `bear intercept -- false`, the exit code is
> non-zero and matches the build's.

Protected by `intercept_exit_code_for_success` and
`intercept_exit_code_for_failure`.

Given a build that is interrupted mid-compile:

> When the user presses Ctrl-C while the compiler is running,
> then the compiler terminates,
> and `bear` exits with a non-zero status.

Coverage pending.

## Notes

- Dedicated integration coverage for the interrupted-mid-compile
  case (as opposed to the interrupted `sleep` case) is the next
  test worth adding under this requirement.
- Related: `interception-preload-mechanism`,
  `interception-wrapper-mechanism`.

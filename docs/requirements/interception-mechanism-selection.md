---
title: Interception mechanism selection
status: implemented
---

## Intent

The user runs a build under Bear without choosing how the interception
happens. Bear has two interception mechanisms: the preload mechanism
(`interception-preload-mechanism`) and the wrapper mechanism
(`interception-wrapper-mechanism`). It selects exactly one of them per
invocation, before the build starts. The default fits the platform; a
configuration option forces either mechanism for users who need the
other one. Whichever runs, the build behaves as it would without Bear.

## Acceptance criteria

- **Given** any build, **when** Bear runs it, **then** exactly one
  mechanism is active, selected before the build starts, and it does not
  change mid-build or per intercepted process.
- **Given** a Linux or BSD host and no interception configuration,
  **when** the user runs a shell script in intercept mode, **then** the
  shell and the processes it spawns are all reported. This shows the
  preload mechanism is active, since the wrapper mechanism never sees
  the shell.
- **Given** a macOS or Windows host and no interception configuration,
  **when** the user runs a build under Bear, **then** the wrapper
  directory is present in the working directory while the build runs.
- **Given** a Linux host and a configuration that forces the wrapper
  mechanism, **when** the user runs a build under Bear, **then** the
  wrapper directory is present in the working directory while the build
  runs.
- **Given** a configuration that forces the preload mechanism on a host
  where library injection is unavailable (Windows, or macOS while System
  Integrity Protection is enabled, checked at startup), **when** the
  user runs a build under Bear, **then** Bear reports an error that
  names the wrapper mechanism as the alternative, and the build is not
  started. Bear never silently switches to the other mechanism.
- **Given** either mechanism, **when** the build runs, **then** its
  output is the same as without Bear. Bear's exit code is owned by
  `cli-exit-codes`.
- **Given** either mechanism and a report that cannot reach Bear,
  **when** the build runs `cc -c test.c`, **then** the compiler still
  runs, the build's output and exit code are unchanged, and the only
  effect is that the entry for `test.c` is missing from the database.
- **Given** either mechanism and a parallel build of four source files
  (`make -j4`), **when** Bear finishes, **then** all four compilations
  appear in the database. No report is lost.

---
title: Diagnostic message format
status: implemented
---

## Intent

Bear prints diagnostics for two readers. A user running a build wants
short, familiar messages that blend into a build log, the way `cc`,
`make`, and `git` print theirs (`cc: error: ...`). A developer debugging
Bear wants every detail. Bear runs as several processes (the driver, the
wrapper, and the preloaded library inside each intercepted compiler), and
their lines interleave on one terminal, so each line must say which
process spoke, when, at what level, and from where in the code.

The user view is the default. Opting into diagnostic logging switches
every line to the developer view. The man page names the setting that
does it.

## Acceptance criteria

- Diagnostics go to standard error. Standard output carries only the
  tool's data (the compilation database or the event stream), so the two
  never mix.
- Every line names the process that emitted it. The names are stable:
  the driver is `bear`, the wrapper is `wrapper`, and the preloaded
  library is `preload`.
- In the user view a line reads `bear: <message>`. A warning adds
  `warning:` after the prefix and an error adds `error:`
  (`bear: warning: <message>`, `bear: error: <message>`).
- In the user view only warnings and errors are shown. Skipped lines of
  shell text and rejected lines of an event stream are warnings. The run
  summary of the output stage is at info level, so it shows only when the
  user opts in.
- Every diagnostic uses this format, errors that stop Bear before the
  build starts included. Errors in the command line itself are the
  exception: they come with the usage text, and their exit code is owned
  by [`cli-exit-codes`](cli-exit-codes.md).
- An error or warning about a file names that file once, with what Bear
  was doing with it and the cause (`bear: error: cannot write <file>: <cause>`).
  It does not name Bear's internal components or stages, and it does not
  repeat the same failure at several levels.
- **Given** parse-sh mode reading shell text with an unsupported line and
  no opt-in to logging, **when** the user runs it, **then** the skip is
  reported on standard error as `bear: warning: <message>`, and standard
  output carries only the database.
- **Given** a build that gives no warnings or errors and no opt-in to
  logging, **when** the user runs it under Bear, **then** Bear writes
  nothing to standard error.
- **Given** the wrapper mechanism and a wrapper that fails to report an
  execution, **when** the failure is logged without opt-in, **then** the
  line starts with `wrapper: error:`.
- **Given** an output path in a directory that does not exist, **when**
  the user runs a build, **then** before the build runs Bear prints one
  line, `bear: error: cannot create <output>: <cause>`.
- **Given** an output that fails while it is written (a full device),
  **when** Bear writes it, **then** Bear prints one line,
  `bear: error: cannot write <output>: <cause>`.
- **Given** a named input file that does not exist, **when** semantic
  mode or parse-sh mode runs, **then** Bear prints one line,
  `bear: error: cannot read <input>: <cause>`.
- **Given** a build command that cannot be started, **when** the user
  runs it under Bear, **then** Bear prints one line,
  `bear: error: cannot run '<command>': <cause>`.
- The messages about a previous database when appending are owned by
  [`output-append`](output-append.md).
- **Given** an opt-in to diagnostic logging, **when** Bear runs, **then**
  every line carries a timestamp, the level, the process name with its
  process id, and the source location in Bear, then the message. The
  requested level sets which lines appear. An empty setting counts as no
  opt-in.
- **Given** an opt-in at debug level and a build whose compiler starts
  subprocesses, **when** the user runs it under the preload mechanism,
  **then** standard error holds lines from `bear` and lines from
  `preload`, each with its own process id, so interleaved lines stay
  apart.

## Non-functional constraints

- All diagnostic text is ASCII.
- The cause part of a message is the operating system's own text where
  the failure comes from it, so its wording varies by platform.
- The format is a contract on standard error only. Changing it does not
  change exit codes or the compilation database.
- The man page ([`../../man/bear.1.md`](../../man/bear.1.md)) documents
  both views for users.

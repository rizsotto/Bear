---
title: Events file as external interchange format
status: implemented
---

## Intent

Intercept mode writes the executions it captures to an events file the
user names. Semantic mode reads an events file and produces a
compilation database. The user expects the events file to be a
documented interchange format: its schema, encoding, and stability are
written down, so outside tools can produce or consume it without
reading Bear's sources. One use is to turn an existing build log into a
compilation database without running the build again.

## Acceptance criteria

Format rules:

- One line of the events file is one JSON object that describes one
  execution. Lines end with `\n`. The file is UTF-8, with no comments
  and no trailing commas.
- Each event object has exactly these four keys, all required. A line
  that misses one, or holds one with the wrong JSON type, does not
  conform.

  | JSON key      | Type                     | Meaning                                                                    |
  |---------------|--------------------------|----------------------------------------------------------------------------|
  | `executable`  | string (filesystem path) | Path to the program run. Absolute, or a bare name as the caller spelled it. |
  | `arguments`   | array of strings         | The argument vector. Element zero is the program name (`argv[0]`).         |
  | `working_dir` | string (filesystem path) | Absolute working directory the program ran in.                             |
  | `environment` | object (string->string)  | Environment variables in effect for the program.                           |

- The stable part is the four key names and their JSON types. Changing
  any of them needs a major-version bump of the format. Across major
  versions, compatibility may break.
- The contents of `environment` are advisory, not stable. Every Bear
  producer keeps only the variables that can change which build tool
  runs or how it compiles. Which variables those are may change between
  releases, with one fixed rule: `PATH`, when captured, is kept, so a
  consumer can resolve bare executable names. A consumer must not assume
  any other variable is present. A third-party producer may include or
  leave out variables freely. The database records the compiler as
  spelled, never resolved (owned by
  [`output-json-compilation-database`](output-json-compilation-database.md)).

Scenarios:

- **Given** an events file from a third-party tool that conforms to the
  schema, **when** semantic mode reads it, **then** the database holds
  one entry per recognized compiler call in the file.
- **Given** an events file from a successful intercept mode run, by the
  same Bear or by an earlier release in the same major version of the
  format, **when** semantic mode reads it, **then** the run succeeds and
  the database equals the one combined mode gives for the same build.
- **Given** the same three compiler events written to one file in order
  and to a second file reversed, **when** semantic mode reads each file,
  **then** both databases hold the same three entries. Only their order
  may differ. Order rules when appending are owned by
  [`output-append`](output-append.md).
- **Given** an events file with one malformed line (invalid JSON, a
  missing key, a wrong type) in the middle, **when** semantic mode reads
  it, **then** Bear reports the line number and the reason, keeps
  reading the lines after it, and writes a database from the valid
  lines. Whether the run fails, for example when every line is rejected,
  is owned by [`cli-exit-codes`](cli-exit-codes.md). Parse-sh mode uses
  the same skip-and-continue rule for its own input
  ([`interception-shell-text-parsing`](interception-shell-text-parsing.md)).
- **Given** a producer piped into semantic mode with no input file
  named, **when** semantic mode runs, **then** it reads the events from
  standard input, and the database holds one entry per recognized
  compiler call. Diagnostics go to standard error.
- **Given** semantic mode reading an empty input, from a named file or
  from standard input, **when** it runs, **then** it exits `0`, writes
  an empty database, and prints a warning on standard error that the
  input held no events.
- **Given** semantic mode reading standard input that is a terminal,
  **when** it starts, **then** it prints a warning on standard error.
  Both warnings are shown by default; levels are owned by
  [`cli-diagnostic-format`](cli-diagnostic-format.md).
- **Given** intercept mode with no events file named, **when** the user
  runs it, **then** Bear exits with a usage error before the build runs.
  There is no default events file.
- **Given** a mode that runs the build (intercept mode, combined mode)
  asked to write its output to standard output, **when** the user runs
  it, **then** Bear exits with an error before the build runs, the error
  says the output cannot go to standard output, and no file named `-`
  is created. The build's own output shares that stream.
- The events file is written all or nothing, as owned by
  [`output-atomic-write`](output-atomic-write.md).

## Non-functional constraints

- The format round-trips: events written by intercept mode are always
  accepted by semantic mode. A break in either direction is a bug.
- The format stays JSON Lines, not a single JSON array, so a producer
  can stream it and a truncated file still yields its complete lines.
- The schema in this file is normative for the events file.

## Rationale

- [event-file-defaults](../rationale/event-file-defaults.md)

---
title: Events file as external interchange format
status: accepted
---

## Intent

Bear's `intercept` mode writes a JSON Lines file of captured executions
(default `events.json`), and Bear's `semantic --input <file>` mode reads
the same file to produce a compilation database. The two modes already
ship and work today. What is missing is a written contract: users and
third-party tooling cannot tell which fields are stable, what guarantees
the format makes, or how to produce a synthetic events file (for example,
to convert an existing build log into a compilation database without
re-running the build).

The user expects the events file to be a documented interchange format:
schema, encoding rules, and stability promise written down so external
tools can produce or consume it without reverse-engineering Bear's
sources.

## Event schema

Each event object has exactly these four keys. All four are required:
a line missing any of them, or carrying one with the wrong JSON type, is
a non-conforming line. The source-of-truth type is `intercept::Execution`
in `crates/intercept/src/lib.rs`.

| JSON key      | Type                     | Meaning                                                                    |
|---------------|--------------------------|----------------------------------------------------------------------------|
| `executable`  | string (filesystem path) | Path to the program run. May be absolute or a bare name resolved via PATH. |
| `arguments`   | array of strings         | The argument vector; element zero is the program name (`argv[0]`).         |
| `working_dir` | string (filesystem path) | Absolute working directory the program ran in.                             |
| `environment` | object (string->string)  | Environment variables in effect for the program.                           |

The stable subset is the four key names and their JSON types: changing
any of them requires a major-version bump of the format. Within that
promise, the *contents* of `environment` are advisory, not stable: Bear
filters captured variables to a build-relevant subset, so a consumer must
not assume any particular variable is present, and a producer may include
or omit variables freely. The `PATH` variable, when present, is what
resolves a bare `executable`; a producer that wants bare names resolved
should supply it.

## Acceptance criteria

- One line of the events file is one JSON object describing a single
  execution event, conforming to the schema above. Lines are
  newline-terminated (`\n`); no comments; no trailing comma; UTF-8
  encoded.
- `bear semantic --input <file>` accepts any file conforming to the
  documented schema. The producer of the file does not need to be Bear.
- `bear semantic --input <file>` is order-independent across lines: the
  same set of events in any order yields a `compile_commands.json` with
  the same set of entries (modulo append-order semantics defined by
  `output-append`).
- A non-conforming line (invalid JSON, missing required field, wrong
  type) is reported with line number and reason, and processing
  continues with subsequent lines. Empty input succeeds with an empty
  database. Non-empty input from which at least one event was accepted
  succeeds. Non-empty input in which every line was rejected exits
  non-zero. (Any additional producer, such as one parsing shell text,
  applies the same skip-and-continue rule to its own input; see
  [`interception-events-from-shell-text`](interception-events-from-shell-text.md).)
- `bear semantic --input -` reads the event stream from standard input,
  and any non-executing producer may write the stream to standard output,
  so the format is pipeable (`<producer> | bear semantic --input -`).
  Diagnostics go to stderr, keeping stdout machine-readable. A mode that
  runs the build does not accept `-` for output: the intercepted build's
  own stdout shares that stream and would corrupt it (a non-atomic write
  can split a JSON line), so `bear intercept` writes events only to a
  file.

## Non-functional constraints

- The format must round-trip: events produced by `bear intercept` must
  always be accepted by `bear semantic --input`. A regression in either
  direction is a bug.
- The format must remain JSON Lines, not a single JSON array. This
  matters for streaming producers and for fault-tolerant readers
  (a truncated file still yields N-1 valid events).
- The wire schema documented in this requirement is normative for the
  events file format.

## Testing

Given a synthetic events file produced by a third-party tool that
conforms to the schema:

> When the user runs `bear semantic --input synthetic.json -o cdb.json`,
> then `cdb.json` contains one compilation entry per recognizable
> compiler invocation in the synthetic file.

Given a Bear-produced events file from a successful `bear intercept`
run:

> When the user runs `bear semantic --input events.json` against it,
> then the resulting compilation database is identical to the one
> produced by an equivalent `bear -- <build>` run.

Given an events file with one malformed line in the middle:

> When the user runs `bear semantic --input broken.json`,
> then Bear reports the line number and parse reason,
> processes the surrounding valid lines,
> and writes a compilation database from the valid subset.

Given an events file produced by Bear vN and consumed by Bear vN+1
within the same major-version line:

> When the user runs `bear semantic --input old-events.json` with the
> newer Bear,
> then the run succeeds and produces an equivalent compilation database.

## Notes

- GitHub issue #644 requested a post-processing mode that turns an
  existing build log into a compilation database. The maintainer
  declined to ship a build-log parser (build-system-specific, out of
  scope), but `bear semantic --input` already provides the consumer
  half. This requirement documents the contract so users can build
  their own log-to-events converters. Bear now also ships one such
  producer on this seam; see
  [`interception-events-from-shell-text`](interception-events-from-shell-text.md).
- Only the consumer side (`semantic --input -`) and non-executing
  producers use `-`. Modes that run the build cannot, because the build's
  own stdout is the same stream. Redirecting the build's stdout to stderr
  to free the channel was considered and rejected: it silently changes
  observable build behavior (tools that detect a tty or write results to
  stdout would break) for a pipeline no producer needs.
- Out of scope: backward compatibility guarantees across major versions;
  those are explicitly allowed to break. A build-log parser is out of
  scope *for this requirement* - it defines only the interchange
  contract; a producer is a separate contract that depends on this one.

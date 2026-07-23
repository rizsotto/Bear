---
title: Diagnostic message format
status: implemented
---

## Intent

Bear prints diagnostics for two different readers, and one format cannot
serve both well.

A user running a build wants terse, familiar messages. Compilers, linkers,
and `make` all prefix a diagnostic with the program name (`cc: error: ...`),
and Bear should read the same way so its output blends into a build log
without explanation.

A developer debugging Bear wants the opposite: every detail. Bear is several
cooperating processes (the driver that supervises the build, the wrapper, and
the preloaded library inside each intercepted compiler), and their messages
interleave on one terminal. To make sense of them a reader needs to know, per
line, which process spoke, when, at what severity, and from where in the code.

So the default is quiet and user-facing; asking for detail switches the whole
output to the developer view.

## Acceptance criteria

- Diagnostics are written to standard error. Standard output carries only the
  tool's data (the compilation database or the event stream), so the two never
  mix. See [`output-json-compilation-database`](output-json-compilation-database.md).

- By default a user sees only warnings and errors. Routine progress detail is
  suppressed unless the user opts into it.

- Every diagnostic line names the process that emitted it. The three process
  identities are stable: the driver is `bear`, the wrapper is `wrapper`, and
  the preloaded library is `preload`.

- By default each line is prefixed with the emitting process identity in the
  UNIX style: `bear: <message>`. A warning adds a `warning:` qualifier and an
  error adds an `error:` qualifier after the prefix (`bear: warning: <message>`,
  `bear: error: <message>`).

- When the user opts into diagnostic logging, the format switches to the
  developer view for every line: a timestamp, the severity, the emitting
  process identity together with its process id, and the source location
  within Bear, followed by the message. Opting in also lowers the threshold so
  levels below warning are shown, and the requested level governs which lines
  appear.

- In the developer view, lines from different processes remain distinguishable
  even when interleaved, because each carries its process identity and a
  distinct process id.

## Non-functional constraints

- All diagnostic text is ASCII.
- The format is a user-facing contract on standard error, not the data output;
  changes to it do not affect exit codes or the compilation database.

## Testing

Given a standalone filter run that skips a line of its input (for example a
`parse-sh` run over an unsupported construct) and no opt-in to logging:

> When the user runs it, then the skip is reported on standard error as
> `bear: warning: <message>`, and standard output still carries only the
> emitted events.

Given a build run under the driver with diagnostic logging opted in at debug
level, compiling a source that makes the compiler launch subprocesses:

> When the user runs the build, then standard error carries lines tagged with
> the `bear` process identity from the driver and lines tagged with the
> `preload` process identity from the intercepted compiler processes, each line
> carrying a timestamp, a severity, a process id, and a source location.

Given the same run without opting into logging and with no warnings or errors
to report:

> When the user runs the build, then Bear writes nothing to standard error.

Given a wrapper-mode build in which the wrapper fails to report an execution:

> When the failure is logged without opting into logging, then the line is
> prefixed `wrapper:` (with the `error:` qualifier), naming the process that
> emitted it.

## Notes

The user view is the default; the developer view is opt-in through a standard
logging environment variable, which also lowers the threshold to the requested
level. An empty value counts as unset. The man page
([`../man/bear.1.md`](../man/bear.1.md)) names that variable and documents both
views for users.

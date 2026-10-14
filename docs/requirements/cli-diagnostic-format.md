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

- Every diagnostic uses this format, including an error that stops Bear
  before the build starts (a configuration that cannot be loaded, an
  output that cannot be created, an input that cannot be found). Errors in
  the command line itself are the exception: they are reported with the
  usage text, as owned by `cli-exit-codes`.

- An error or warning about a file names that file once, together with
  what Bear was doing with it and the cause
  (`bear: error: cannot write <file>: <cause>`). It does not name Bear's
  internal components or processing stages, and it does not repeat the
  same failure at several levels.

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
- The cause part of a message is the operating system's own text where
  the failure comes from it, so its wording varies by platform.
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

Given an output path in a directory that does not exist:

> When the user runs a build, then before the build runs Bear prints one
> line, `bear: error: cannot create <output>: <cause>`, and exits non-zero.

Given an output that fails while it is written (e.g. a full device):

> When the user runs Bear, then it prints one line,
> `bear: error: cannot write <output>: <cause>`, and exits non-zero.

Given a named events input that does not exist:

> When the user runs the analysis, then Bear prints one line,
> `bear: error: cannot read <input>: <cause>`, and exits non-zero.

Given a build command that cannot be started:

> When the user runs the build, then Bear prints one line,
> `bear: error: cannot run '<command>': <cause>`, and exits non-zero.

Given append mode and an existing database that is not valid JSON:

> When the user runs a build, then Bear prints a warning,
> `bear: warning: cannot read previous entries from <database>: <cause>`,
> and still writes the new entries.

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

---
title: Execution events from shell command text
status: implemented
---

## Intent

A user has the text a build system prints when asked to show commands
without running them - for example `make -n` output, or a saved build
log - and wants a compilation database from it without paying for a full
build. They expect a mode that reads that text and emits the execution
event stream defined by
[`interception-events-format`](interception-events-format.md), which the
existing `semantic` mode then turns into a compilation database:

```sh
make -n | bear <this mode> | bear semantic
```

This is a best-effort front-end, not a replacement for interception.
Interception observes real `exec()` calls and remains the high-fidelity,
recommended path; this mode reconstructs approximate events from text a
build system chose to print. The user must be told, in the man page and
in this contract's known limitations, that fidelity is bounded by what
the dry-run text contains.

## Acceptance criteria

- The mode reads shell command text from standard input or from a named
  file, and writes the event stream from
  [`interception-events-format`](interception-events-format.md) to
  standard output or a named file. Standard input and standard output
  are the defaults, so the mode is a plain filter.
- Each line that lexes as a simple command becomes one execution event,
  with one exception: `cd` (see working-directory tracking below). The
  mode does not decide what a compiler is; `mv`, `ar`, `ln`, and `mkdir`
  lines produce valid events just as compiler lines do. Filtering
  non-compiler commands is the consumer's job, not this producer's.
- The `executable` field is the first word that is not a leading
  assignment, taken verbatim. Bare names stay bare; the consumer resolves
  them. The `arguments` field carries that word as element zero, matching
  interception.
- Leading `VAR=value` words are captured as environment overrides for
  that command, not treated as the executable.
- The supported shell subset is observable and limited to:
  - word splitting on unquoted whitespace; single quotes, double quotes,
    and backslash escapes; line continuations (backslash-newline);
  - command separators `newline`, `;`, `&&`, `||`, `&`, and `|`, where
    each side of a pipe is a candidate command;
  - comments (`#` to end of line) and blank lines, which produce nothing;
  - redirections (`>`, `>>`, `<`, `2>`, `2>&1`, `>/dev/null`, and the
    like), which are recognized and removed from the command's words. A
    redirect's whole target word is discarded uninspected: an expansion
    or substitution inside one does not skip the line. An unterminated
    quote or unterminated substitution in a target skips the line loudly.
  - brace groups `{ ...; }` in command position, which run in the
    current shell. The braces are structural and produce no event; the
    commands between them are parsed with the shared shell state, so a
    `cd` inside a group persists after the closing brace, matching `sh`.
    Groups may nest and may span lines. An unmatched `}` (a close with
    no open group) skips its line loudly, and an input that ends with a
    `{` still open is reported loudly against the opening line; both are
    reported as unbalanced braces. This is a subshell contrast, not a
    generalization: `( ... )` runs in a child shell and stays
    unsupported.
- Any construct outside that subset - subshell groups `( ... )`, command
  substitution (backticks or `$(...)`), parameter expansion (`$VAR`,
  `${...}`), globs in the executable word, here-documents, and the
  `case`/`for`/`while`/`if` keywords - causes the whole line to be
  skipped. A skip is reported on standard error with the line number and
  the reason. The mode never guesses at the meaning of an unsupported
  construct.
- The working directory of each event is tracked:
  - it starts at the mode's own working directory, or at a caller-set
    value when the input came from elsewhere (such as a CI log);
  - a `cd <dir>` command updates it for subsequent commands, and is
    consumed only for that effect: `cd` produces no execution event,
    because it is a shell builtin that never reaches `exec()` and so would
    not appear in an intercepted stream. `cd -` restores the previous
    working directory when one is known and is a loud skip otherwise;
    the `-` is never treated as a directory name;
  - `make[N]: Entering directory '...'` and `Leaving directory '...'`
    markers push and pop it, so recursive `make -n -w` output tracks
    directories correctly. These markers are an explicit, documented
    extension to the shell subset.
- The environment of each event is the mode's own environment overlaid
  with the line's leading `VAR=value` assignments, then reduced by the
  shared build-relevant filter (including its `PATH` guarantee) owned by
  [`interception-events-format`](interception-events-format.md).
- The mode emits the raw event stream and never consults Bear's
  configuration; configuration shapes only the downstream semantic
  analysis that turns the stream into a compilation database, not this
  producer. Supplying a configuration file to this mode is therefore
  rejected with an error rather than silently accepted and ignored, so
  the user is not misled into believing it took effect.
- Skipped lines are reported on standard error (line number and reason),
  and when any line is skipped a summary count is printed there too; this
  reporting is on by default and does not require a logging opt-in. A run
  with nothing skipped stays quiet, and empty input still emits a stderr
  notice. Whether such a run succeeds or exits non-zero -- following the
  same skip-and-continue rule as
  [`interception-events-format`](interception-events-format.md) -- is the
  exit-code contract's concern; see [`cli-exit-codes`](cli-exit-codes.md).

## Known limitations

Non-guarantees, which hold regardless of parser quality because they are
inherent to dry-run text. These must also appear in the man page:

- Dry-run output can omit commands. Recursive make may not propagate the
  dry-run flag; commands behind not-yet-generated sources never appear
  because the generator did not run; `$(shell ...)` evaluated at parse
  time can differ from a real build.
- The build system must both support a dry-run mode and print real
  commands in it. Silent rules, custom launchers, and response files
  reduce fidelity.
- The reconstructed execution context is approximate. The environment and
  the `PATH` used to resolve bare names are the mode's own at parse time
  and may differ from a real build's. Parsing a log captured on another
  machine implies a foreign environment and PATH that this mode cannot
  reproduce, even when the caller pins the working directory.
- Input is POSIX `sh` command text only. Non-POSIX shells and Windows
  `cmd` are out of scope. A saved full-build log is accepted only insofar
  as its lines happen to be `sh` commands; interleaved compiler output is
  skipped loudly like any other unsupported line.
- Whether a source file exists on the parsing machine does not affect the
  output: entries are reconstructed from the parsed text alone, so a log
  whose sources are absent (a CI log parsed elsewhere) still produces a
  full database. The one exception is Bear's canonicalizing path format,
  which resolves paths against the real filesystem and therefore needs the
  sources present; the man page documents the existence-free alternative
  for that case.

## Testing

Given real `make -n` output from a configured project (the zlib fixture):

> When the user runs the mode over it and pipes the result to
> `bear semantic --input -`, then the compilation database covers every
> compiler invocation in the input, non-compiler lines (`ar`, `mv`,
> `mkdir`, `ln`) produce no compilation entries, and the skipped subshell
> and redirect lines are reported on stderr but are not fatal.

Given a line with a leading assignment and a redirection, such as
`CC=gcc gcc -c foo.c -o foo.o >/dev/null 2>&1`:

> When the mode parses it, then the event has `executable` `gcc`,
> `arguments` beginning with `gcc`, the redirection removed from the
> arguments, and `CC=gcc` overlaid on the environment.

Given a recursive-make log carrying `Entering directory` and
`Leaving directory` markers around `cd`-free compiler lines:

> When the mode parses it, then each event's `working_dir` reflects the
> directory in effect at that line.

Given input in which every line is an unsupported construct (all
subshells, all command substitutions):

> When the mode runs, then it reports each skipped line on stderr, emits
> no events, and exits non-zero.

Given an invocation of this mode that also supplies a configuration file:

> When the user runs it, then the invocation is rejected with an error
> stating that configuration does not apply to this mode, and no event
> stream is produced.

## Notes

- GitHub issue #284 ("Dry run flag") is the recurring request; #287,
  #219, #644, #456, and #404 ask for the same capability from build-log
  and dry-run angles.
- The name "dry run" describes the build system's mode, not this mode:
  the build system dry-runs, this mode only parses the text. The man page
  should carry the "dry run" vocabulary so users searching for it arrive
  here.

## Rationale

- [parse-sh-producer](../rationale/parse-sh-producer.md)
- [parse-sh-single-tokenizer](../rationale/parse-sh-single-tokenizer.md)

---
title: Compilation database from shell command text
status: accepted
---

## Intent

A user has the text a build system prints when asked to show commands
without running them - for example `make -n` output, or a saved build
log - and wants a compilation database from it without paying for a full
build, in one step:

```sh
make -n | bear parse-sh
```

The mode reconstructs the executions the text describes and feeds them
through the same semantic analysis and output stage as every other
database-producing mode; the compilation database is its only product.

This is a best-effort front-end, not a replacement for interception.
Interception observes real `exec()` calls and remains the high-fidelity,
recommended path; this mode reconstructs approximate executions from
text a build system chose to print. The user must be told, in the man
page and in this contract's known limitations, that fidelity is bounded
by what the dry-run text contains.

## Acceptance criteria

Mode shape:

- The mode reads shell command text from standard input or from a named
  file; standard input is the default.
- It writes a compilation database whose content, format, and
  destination handling are governed by the same output contracts as the
  other database-producing modes
  ([`output-json-compilation-database`](output-json-compilation-database.md),
  [`output-append`](output-append.md),
  [`output-atomic-write`](output-atomic-write.md), and the rest of the
  `output-` set). The destination defaults to `compile_commands.json`,
  and appending to an existing database is supported.
- Because the mode runs no build, the user may direct the database to
  standard output; diagnostics stay on standard error. A streamed
  database is written directly (atomic file replacement does not apply)
  and cannot be combined with appending; asking for both is a usage
  error.
- The user's configuration applies exactly as in the other
  database-producing modes: it shapes compiler recognition, duplicate
  detection, filtering, and formatting of the database. Configuration
  never changes how the text is parsed.
- The working directory the reconstruction starts in can be overridden
  by the caller, for text captured elsewhere (such as a CI log).
- The reconstructed executions are an internal intermediate; the mode
  does not expose them (see known limitations).

Parsing contract:

- Each line that lexes as a simple command yields one candidate
  execution, with one exception: `cd` (see working-directory tracking
  below). The parser does not decide what a compiler is: `mv`, `ar`,
  `ln`, and `mkdir` lines are valid candidates just as compiler lines
  are. Which candidates become database entries is decided downstream by
  compiler recognition and output validation, exactly as for intercepted
  executions; a candidate that is not recognized as a compiler produces
  no entry and no diagnostic. Only lines the parser cannot handle are
  skipped loudly (see skip reporting below).
- A candidate's executable is the first word that is not a leading
  assignment, taken verbatim. Bare names stay bare, and the database
  records the observed spelling (owned by
  [`output-json-compilation-database`](output-json-compilation-database.md)).
- Leading `VAR=value` words are captured as environment overrides for
  that command, not treated as the executable. A candidate's environment
  is the mode's own environment at parse time overlaid with those
  assignments, and it participates in semantic analysis the same way an
  intercepted execution's environment does - observably through
  environment-derived flags
  ([`output-env-derived-flags`](output-env-derived-flags.md)).
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
    current shell. The braces are structural and produce no candidate;
    the commands between them are parsed with the shared shell state, so
    a `cd` inside a group persists after the closing brace, matching
    `sh`. Groups may nest and may span lines. An unmatched `}` (a close
    with no open group) skips its line loudly, and an input that ends
    with a `{` still open is reported loudly against the opening line;
    both are reported as unbalanced braces. This is a subshell contrast,
    not a generalization: `( ... )` runs in a child shell and stays
    unsupported.
- Any construct outside that subset - subshell groups `( ... )`, command
  substitution (backticks or `$(...)`), parameter expansion (`$VAR`,
  `${...}`), globs in the executable word, here-documents, and the
  `case`/`for`/`while`/`if` keywords - causes the whole line to be
  skipped. A skip is reported on standard error with the line number and
  the reason. The mode never guesses at the meaning of an unsupported
  construct.
- The working directory of each candidate is tracked, and reaches the
  database as each entry's directory:
  - it starts at the mode's own working directory, or at a caller-set
    value when the input came from elsewhere (such as a CI log);
  - a `cd <dir>` command updates it for subsequent commands, and is
    consumed only for that effect: `cd` produces no candidate, because
    it is a shell builtin that never reaches `exec()` and so would never
    be observed by interception either. `cd -` restores the previous
    working directory when one is known and is a loud skip otherwise;
    the `-` is never treated as a directory name;
  - `make[N]: Entering directory '...'` and `Leaving directory '...'`
    markers push and pop it, so recursive `make -n -w` output tracks
    directories correctly. These markers are an explicit, documented
    extension to the shell subset.
- Skipped lines are reported on standard error (line number and reason),
  and when any line is skipped a summary count is printed there too; this
  reporting is on by default and does not require a logging opt-in. A run
  with nothing skipped stays quiet, and empty input still emits a stderr
  notice. Whether a run succeeds or exits non-zero is the exit-code
  contract's concern; see [`cli-exit-codes`](cli-exit-codes.md).

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

Deliberate exclusion, likely to be proposed again:

- The mode offers no machine-readable view of the reconstructed
  executions: the compilation database is its only output, so parsed
  non-compiler commands (`ar`, `ln`, a linker line) appear nowhere. The
  skip diagnostics on standard error are the only window into the
  reconstruction. The reasoning is recorded in
  [parse-sh-database-product](../rationale/parse-sh-database-product.md);
  a public execution-stream output can be added compatibly if real
  demand appears.

## Testing

Given real `make -n` output from a configured project (the zlib fixture):

> When the user pipes it to `bear parse-sh`, then `compile_commands.json`
> is written at the default destination and covers every compiler
> invocation in the text, non-compiler lines (`ar`, `mv`, `mkdir`, `ln`)
> produce no entries and no diagnostics, and the lines outside the
> supported subset (subshells, substitutions) are reported on stderr as
> skips but are not fatal.

Given the line `gcc -c foo.c` saved in a file named `build.sh`:

> When the user runs the mode with `build.sh` as the named input and
> `db.json` as the named output, then `db.json` contains exactly one
> entry: its file is `foo.c`, its compiler is spelled `gcc`, and its
> directory is the mode's working directory. Standard output carries
> nothing.

Given the zlib fixture and a configuration that widens duplicate
detection to compare full argument lists:

> When the user supplies that configuration to the mode, then the
> database keeps the entries the default de-duplication folds away - the
> configuration shaped the semantic stage of this mode just as it shapes
> the other database-producing modes. The stderr skip report is
> identical with and without the configuration: configuration never
> changes how the text is parsed.

Given input whose only line is the non-compiler command `mv objs/a.o a.lo`:

> When the mode parses it, then no line is skipped, no diagnostic names
> the line, and the database is written empty - a clean parse with no
> recognized compiler is not an error (the exit code is owned by
> [`cli-exit-codes`](cli-exit-codes.md)).

Given input exercising the supported subset together - quotes, an
escape, a continuation, separators including a pipe, redirections, a
comment, and a blank line:

```sh
gcc -DNAME='a b' -c \
  foo\ bar.c && ar rc libz.a foo.o | tee log  # archive it

gcc -c baz.c >/dev/null 2>&1 || true
```

> When the mode parses it, then the database has exactly two entries:
> one for `foo bar.c` whose arguments carry the single word
> `-DNAME=a b` (the continuation joined, quoting and escape resolved),
> and one for `baz.c` whose arguments carry no redirection words. The
> `ar`, `tee`, and `true` candidates produce no entries, the comment and
> the blank line produce nothing, and no line is skipped.

Given the redirect-target pair `gcc -c foo.c > $(logdir)/x.log` and
`cmd > 'oops`:

> When the mode parses them, then the first yields a clean entry for
> `foo.c` and no skip - the substitution lies entirely inside the
> discarded target - while the second yields nothing and is skipped
> loudly as an unterminated quote.

Given a brace group followed by another command, starting in `/build`:

```sh
{ cd sub; gcc -c a.c; }
gcc -c b.c
```

> When the mode parses it, then the braces produce no entry of their own
> and both compilations appear with directory `/build/sub` - the group's
> `cd` persists past the closing brace. A `}` with no open group
> (`echo a; }`) skips its line loudly, and an input that ends with a `{`
> still open reports unbalanced braces against the line that `{` opened
> on.

Given the line `gcc -c foo.c` from a log captured elsewhere, and a
caller-set initial working directory of `/custom/build`:

> When the mode parses it, then the entry's directory is
> `/custom/build`, not the mode's own working directory.

Given `cd`-driven directory changes, starting in `/build`:

```sh
cd sub
gcc -c a.c
cd -
gcc -c b.c
```

> When the mode parses it, then neither `cd` line produces an entry,
> `a.c` is recorded with directory `/build/sub`, and `b.c` back in
> `/build`. An input whose first command is `cd -` (no previous
> directory is known) has that line skipped loudly and the working
> directory left unchanged.

Given the line
`CPATH=/opt/include gcc -c foo.c -o foo.o >/dev/null 2>&1` and
environment-derived flags at their default (enabled):

> When the mode parses it, then the entry for `foo.c` names the compiler
> `gcc` (the assignment is not the executable), its arguments carry no
> redirection words, and they include an explicit include flag for
> `/opt/include` - the leading assignment reached semantic analysis as
> that command's environment.

Given a recursive-make log carrying `Entering directory` and
`Leaving directory` markers around `cd`-free compiler lines:

> When the mode parses it, then each entry's directory reflects the
> directory in effect at that line.

Given input in which every line is an unsupported construct (all
subshells, all command substitutions):

> When the mode runs, then it reports each skipped line on stderr and
> the run fails (the exit code is owned by
> [`cli-exit-codes`](cli-exit-codes.md)).

Given input mixing one supported compile line with one unsupported line
(`gcc -c foo.c`, then `for f in *.c; do gcc -c $f; done`):

> When the mode runs, then stderr carries the per-line skip report and a
> summary counting one command parsed and one line skipped - the `for`
> line counts once, not once per command segment inside it. Empty input
> produces a stderr notice that no commands were found; input with
> nothing skipped keeps stderr quiet.

Given a request to stream the database:

> When the user directs the database to standard output, then the JSON
> database appears on standard output with diagnostics kept on standard
> error, and no file is created. Combining standard-output streaming
> with appending is rejected as a usage error.

Given an existing compilation database and new dry-run text:

> When the user runs the mode against the same destination with
> appending enabled, then the result is the merged set governed by
> [`output-append`](output-append.md).

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
- [parse-sh-database-product](../rationale/parse-sh-database-product.md)

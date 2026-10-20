---
title: Compilation database from shell command text
status: implemented
---

## Intent

A user has the text a build system prints when asked to show commands
without running them, such as `make -n` output or a saved build log.
The user wants a compilation database from it in one step, without
paying for a full build:

```sh
make -n | bear parse-sh
```

Parse-sh mode rebuilds the executions the text describes. They go
through the same semantic analysis and output stage as in every other
mode that writes a database. The database is the mode's only product.

This is a best-effort front-end, not a replacement for interception.
Interception sees real `exec()` calls and stays the recommended path.
Parse-sh mode rebuilds approximate executions from text the build
system chose to print, so its fidelity is bounded by that text. The man
page and the known limitations below must say so.

## Acceptance criteria

### Mode shape

- The mode reads shell text from standard input by default, or from a
  named file.
- The database follows the same output contracts as the other modes
  that write one ([`output-json-compilation-database`](output-json-compilation-database.md),
  [`output-append`](output-append.md),
  [`output-atomic-write`](output-atomic-write.md), and the rest of the
  `output-` set). The default destination is `compile_commands.json`.
- Streaming the database to standard output is owned by
  [`output-json-compilation-database`](output-json-compilation-database.md).
- **Given** an existing database and new shell text, **when** the user
  runs the mode against the same destination while appending, **then**
  the result is the merged set owned by [`output-append`](output-append.md).
- **Given** the zlib fixture and a configuration that widens duplicate
  detection to compare full argument lists, **when** the user supplies
  it, **then** the database keeps the entries the default duplicate
  detection folds away, and the skip report on standard error is the
  same with and without it. Configuration shapes recognition, duplicate
  detection, filtering, and formatting as in the other modes. It never
  changes how the text is parsed.
- **Given** the line `gcc -c foo.c` from a log captured elsewhere and a
  caller-set initial working directory of `/custom/build`, **when** the
  mode parses it, **then** the entry's directory is `/custom/build`, not
  the mode's own working directory.
- The mode offers no view of the rebuilt executions. Parsed commands
  that are not compiler calls (`ar`, `ln`, a linker line) appear
  nowhere. The skip warnings on standard error are the only window into
  the parse.

### Candidates

- **Given** the line `gcc -c foo.c` in a file named `build.sh`, **when**
  the user runs the mode with `build.sh` as input and `db.json` as
  output, **then** `db.json` holds exactly one entry: file `foo.c`,
  compiler spelled `gcc`, directory the mode's working directory.
  Standard output carries nothing.
- Each line that lexes as a simple command yields one candidate
  execution, except `cd` (see the table). The parser does not decide
  what a compiler is. Compiler recognition and output validation decide
  which candidates become entries, as for intercepted executions. A
  candidate that is not a compiler gives no entry and no diagnostic.
- **Given** input whose only line is `mv objs/a.o a.lo`, **when** the
  mode parses it, **then** no line is skipped, no diagnostic names the
  line, and the database is written empty. The exit code is owned by
  [`cli-exit-codes`](cli-exit-codes.md).
- A candidate's executable is the first word that is not a leading
  assignment, taken as written. Bare names stay bare.
- A candidate's environment is the mode's own environment at parse time,
  with that command's leading `VAR=value` words laid over it. It reaches
  semantic analysis as an intercepted execution's environment does,
  seen through [`output-env-derived-flags`](output-env-derived-flags.md).
- **Given** the line `CPATH=/opt/include gcc -c foo.c -o foo.o >/dev/null 2>&1`
  and environment-derived flags enabled (the default), **when** the mode
  parses it, **then** the entry names the compiler `gcc`, its arguments
  carry no redirection words, and they include an include flag for
  `/opt/include`.
- The working directory of each candidate is tracked and becomes the
  entry's directory. It starts at the mode's own working directory, or
  at the caller-set one.

### Shell subset

The mode handles a small, fixed subset of POSIX `sh`. A construct marked
skipped makes the whole line skipped: nothing on that line becomes a
candidate, and the line is reported (see skip reporting). The mode never
guesses at the meaning of a skipped construct.

| Construct | Handling |
|---|---|
| Word splitting on unquoted whitespace | Handled |
| Single quotes, double quotes, backslash escapes | Handled |
| Line continuation (backslash-newline) | Handled. The lines join into one |
| Separators: newline, `;`, `&&`, `\|\|`, `&`, `\|` | Handled. Each command is a candidate, both sides of a pipe included |
| Comments (`#` to end of line), blank lines | Handled. They produce nothing |
| Leading `VAR=value` words | Handled. Environment for that command, never the executable |
| Redirections (`>`, `>>`, `<`, `2>`, `2>&1`, `>/dev/null`, and the like) | Handled. Removed from the words. The target word is discarded unread, so an expansion or substitution inside it does not skip the line |
| Unterminated quote or substitution in a redirection target | Skipped |
| Brace group `{ ...; }` in command position | Handled. The braces give no candidate. The commands inside share the shell state, so a `cd` inside persists after `}`, as in `sh`. Groups may nest and span lines |
| `}` with no open group | Skipped, as unbalanced braces |
| Input ending with a `{` still open | Reported as unbalanced braces against the line the `{` opened on. The commands inside the group are kept |
| `cd <dir>` | Handled. Updates the working directory for later commands and gives no candidate, since `cd` is a builtin that interception never sees |
| `cd -` | Handled when a previous directory is known: restores it. Skipped otherwise, with the working directory unchanged. The `-` is never a directory name |
| make's `Entering directory '...'` and `Leaving directory '...'` markers | Handled, as an explicit extension to `sh`. They push and pop the working directory, so recursive `make -n -w` output tracks directories |
| Subshell `( ... )` | Skipped. It runs in a child shell, unlike a brace group |
| Command substitution (backticks, `$(...)`) | Skipped |
| Parameter expansion (`$VAR`, `${...}`) | Skipped |
| Glob in the executable word | Skipped |
| Here-document | Skipped |
| Shell keyword (`case`, `for`, `while`, `if`, and the like) in command position | Skipped |
| Unterminated quote | Skipped |

- **Given** this input,

  ```sh
  gcc -DNAME='a b' -c \
    foo\ bar.c && ar rc libz.a foo.o | tee log  # archive it

  gcc -c baz.c >/dev/null 2>&1 || true
  ```

  **when** the mode parses it, **then** the database has exactly two
  entries: `foo bar.c` with the single argument word `-DNAME=a b`, and
  `baz.c` with no redirection words. `ar`, `tee`, and `true` give no
  entries, the comment and the blank line give nothing, and no line is
  skipped.
- **Given** the lines `gcc -c foo.c > $(logdir)/x.log` and
  `cmd > 'oops`, **when** the mode parses them, **then** the first gives
  an entry for `foo.c` and no skip, and the second gives nothing and is
  skipped as an unterminated quote.
- **Given** this input, starting in `/build`,

  ```sh
  { cd sub; gcc -c a.c; }
  gcc -c b.c
  ```

  **when** the mode parses it, **then** the braces give no entry and
  both compilations have directory `/build/sub`. A `}` with no open
  group (`echo a; }`) skips its line, and an input that ends with a `{`
  still open reports unbalanced braces against the line it opened on
  and still keeps the commands inside the group.
- **Given** this input, starting in `/build`,

  ```sh
  cd sub
  gcc -c a.c
  cd -
  gcc -c b.c
  ```

  **when** the mode parses it, **then** neither `cd` line gives an
  entry, `a.c` has directory `/build/sub`, and `b.c` has `/build`. An
  input whose first command is `cd -` has that line skipped and the
  working directory left unchanged.
- **Given** a recursive-make log with `Entering directory` and
  `Leaving directory` markers around compiler lines without `cd`,
  **when** the mode parses it, **then** each entry's directory is the
  one in effect at that line.

### Skip reporting

- Each skipped line is reported on standard error as a warning, with its
  line number and reason. When any line is skipped, a summary warning
  counts the commands parsed and the lines skipped. Warnings are shown
  by default; levels are owned by
  [`cli-diagnostic-format`](cli-diagnostic-format.md).
- **Given** `gcc -c foo.c` followed by `for f in *.c; do gcc -c $f; done`,
  **when** the mode runs, **then** standard error carries the skip
  warning and a summary of one command parsed and one line skipped. The
  `for` line counts once, not once per command inside it.
- **Given** input with nothing skipped, **when** the mode runs, **then**
  it prints no warning. Its summary is at info level, shown only when
  the user opts in.
- **Given** empty input, **when** the mode runs, **then** it prints a
  warning that no commands were found.
- **Given** real `make -n` output from the zlib fixture, **when** the
  user pipes it to the mode, **then** `compile_commands.json` is written
  at the default destination and covers every compiler call in the
  text, the non-compiler lines (`ar`, `mv`, `mkdir`, `ln`) give no
  entries and no diagnostics, and the lines outside the subset
  (subshells, substitutions) are reported as skips but are not fatal.
- **Given** input in which every line is skipped, **when** the mode
  runs, **then** it reports each line and the run fails. The exit code
  is owned by [`cli-exit-codes`](cli-exit-codes.md).

## Known limitations

These hold however good the parser is, because they come from the
dry-run text itself. The man page must state them too.

- Dry-run output can leave out commands. Recursive make may not pass
  the dry-run flag on. Commands behind sources that are not generated
  yet never appear, because the generator did not run. `$(shell ...)`
  evaluated at parse time can differ from a real build.
- The build system must support a dry run and print real commands in
  it. Silent rules, custom launchers, and response files lower fidelity.
- The rebuilt execution context is approximate. The environment and the
  `PATH` used to resolve bare names are the mode's own at parse time and
  may differ from the real build's. A log from another machine implies a
  foreign environment and `PATH` that the mode cannot rebuild, even when
  the caller sets the working directory.
- Input is POSIX `sh` text only. Other shells and Windows `cmd` are out
  of scope. A saved full-build log works only as far as its lines are
  `sh` commands. Interleaved compiler output is skipped like any other
  unsupported line.
- Whether a source file exists on the parsing machine does not change
  the output: entries come from the text alone, so a log whose sources
  are absent still gives a full database. The exception is the path
  format that resolves paths against the real filesystem. It needs the
  sources present. The man page names the alternative for that case.

## Rationale

- [parse-sh: a quarantined, best-effort parser whose only product is the database](../rationale/parse-sh-producer.md)
- [A single streaming tokenizer for parse-sh](../rationale/parse-sh-single-tokenizer.md)

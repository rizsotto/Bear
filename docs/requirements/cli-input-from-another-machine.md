---
title: Input from another machine
status: implemented
---

## Intent

A user has a build log or an event file captured on another machine: a
CI runner, a container, a colleague's workstation. The build's sources
and compilers are not on this machine, or are different here. The user
expects the database that input describes, with nothing from this
machine mixed in.

An option tells Bear that the input comes from another machine. With
it, the database depends only on the input and the configuration. Bear
opens none of the build's files, runs none of its programs, and reads
nothing from its own environment to decide what an entry holds.

The option is offered in the modes that do not run the build. A mode
that runs the build sees the file system the build used, so its checks
give the right answer there.

## Acceptance criteria

Scope:

- **Given** semantic mode or parse-sh mode, **when** the option is
  given, **then** Bear accepts it.
- **Given** combined mode or intercept mode, **when** the option is
  given, **then** the command line is malformed. The exit code is owned
  by [`cli-exit-codes`](cli-exit-codes.md).
- **Given** semantic mode asked to list the compilers it recognizes,
  **when** the option is given, **then** the command line is malformed.

Source files:

- **Given** a parse-sh run on a log whose sources are not on this
  machine, no configuration file, and the option, **when** Bear writes
  the database, **then** it holds an entry for every compilation in the
  log.
- **Given** the same run with a configuration that turns the
  missing-source filter on explicitly, **when** Bear writes the
  database, **then** it holds the same entries. The filter
  ([`output-missing-source-filter`](output-missing-source-filter.md))
  is not applied, and no diagnostic names it.
- **Given** an existing database with an entry whose source file is not
  on this machine, and a run that is appending with the option, **when**
  Bear finishes, **then** that entry is kept.

Compilers:

- **Given** a configuration that names a compiler at a path that does
  not exist on this machine, with a family, **when** Bear runs with the
  option, **then** the configuration is accepted, and an execution of
  the bare file name of that path is parsed with the configured family.
- **Given** a configuration that excludes a compiler at a path that
  does not exist on this machine, **when** Bear runs with the option,
  **then** the configuration is accepted.
- **Given** a configured compiler path, **when** Bear runs with the
  option, **then** an execution matches it only by the path as written,
  or by its file name when the build invoked it by that bare name. No
  path is resolved on this machine to find a match ([`recognition-compiler-names`](recognition-compiler-names.md)).
- **Given** an execution of `cc -Xclang -ast-dump -c a.c` with no
  configured family for `cc`, **when** Bear runs with the option,
  **then** no program is run to identify `cc`, and the call is parsed
  with Clang flag semantics: one entry for `a.c`, with `-ast-dump` read
  as the argument of `-Xclang`. `c++` and `CC` behave the same way.
- **Given** a configured family for `cc`, **when** Bear runs with the
  option, **then** a `cc` execution is parsed with that family.

Environment:

- **Given** a parse-sh run with the option, while Bear's own
  environment sets a variable that the compiler reads for flags (an
  include path, for example), **when** Bear writes the database,
  **then** no entry holds a flag derived from that variable. A leading
  `VAR=value` word on a command line still applies to that command
  ([`interception-shell-text-parsing`](interception-shell-text-parsing.md),
  [`output-env-derived-flags`](output-env-derived-flags.md)).

Configuration that needs the build's files:

- **Given** a configuration that turns on a feature that reads the
  build's files to shape an entry, **when** Bear runs with the option,
  **then** Bear exits non-zero before it reads the input, and the
  diagnostic names the configuration option and the configuration file.
  The features are: the symlink-resolving path format for `directory`
  or for `file` ([`output-path-format`](output-path-format.md)), header
  entries with either strategy
  ([`output-header-entries`](output-header-entries.md)), and
  response-file inlining
  ([`output-response-file-inlining`](output-response-file-inlining.md)).
  The diagnostic format is owned by
  [`cli-diagnostic-format`](cli-diagnostic-format.md).

## Known limitations

- A `cc`, `c++`, or `CC` that was GCC on the other machine is parsed
  with Clang flag semantics. Clang's flag table includes GCC's, so the
  entry is the same, except for a rare GCC argument whose spelling is
  also a Clang-only flag with another arity. A configured family avoids
  this.
- In parse-sh mode, commands before the first directory change in the
  log, and every command of a log without directory changes, run in
  the initial working directory. Unless the caller sets it to the
  directory as it was on the other machine, it is the directory Bear
  runs in.
- A configuration file found automatically applies as usual. When it
  turns on one of the features above, the user points Bear at another
  configuration file to use the option.

## Rationale

- [One option for input from another machine](../rationale/input-from-another-machine-switch.md)

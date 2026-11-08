---
title: Finding and reading the configuration file
status: implemented
---

## Intent

A user who keeps a configuration file next to the project, or one for
all projects in the user's own configuration directory, expects Bear to
find it without being told. A user who names a file on the command line
expects that file and no other. A file Bear cannot use stops the run
before any work starts, so a typo never turns into a silently different
database.

## Acceptance criteria

Which file is used:

- **Given** a file named on the command line, **when** Bear runs,
  **then** it reads that file, and no file found by search is read.
- **Given** no file named on the command line, **when** Bear runs,
  **then** it searches for a file named `bear.yml`: first in the
  directory Bear runs in, then in the user's configuration directory
  (on Unix the XDG base directory, on Windows the local and then the
  roaming application data directory), each directly and in a `Bear`
  subdirectory. The first file found is used. The man page lists the
  exact locations.
- **Given** a `bear.yml` in the directory Bear runs in and another in
  the user's configuration directory, **when** Bear runs, **then** only
  the one in the directory Bear runs in is read. Files are never
  merged.
- **Given** an environment variable that names a configuration
  directory but is set to an empty value, **when** Bear searches,
  **then** the variable counts as unset.
- **Given** no file named and none found, **when** Bear runs, **then**
  the built-in defaults apply and no diagnostic says so.
- **Given** semantic mode asked to list the compilers it recognizes,
  **when** it runs, **then** no configuration file is read, and naming
  one is a malformed command line.

A file Bear cannot use:

- **Given** a file named on the command line that does not exist or
  cannot be read, **when** Bear runs, **then** it exits non-zero before
  any work starts, and the diagnostic names the file.
- **Given** a file found by search that is not valid YAML, or does not
  match the configuration format, **when** Bear runs, **then** it exits
  non-zero before any work starts, and the diagnostic names the file.
  A broken file is never skipped in favor of the next location.
- **Given** a file that names a schema version Bear does not support,
  **when** Bear runs, **then** it exits non-zero, and the diagnostic
  names the supported version. A file that names no schema version is
  a file that does not match the configuration format.

The exit code is owned by [`cli-exit-codes`](cli-exit-codes.md), and
the diagnostic format by [`cli-diagnostic-format`](cli-diagnostic-format.md).
Which settings a valid file may hold, and what each does, is owned by
the requirement of the behavior it configures.

## Known limitations

- How Bear treats a key it does not know is not settled here.

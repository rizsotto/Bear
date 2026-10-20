---
title: All-or-nothing output files
status: implemented
---

## Intent

An output file Bear writes is never left corrupt or half written, even
when Bear is interrupted or fails. A tool that reads
`compile_commands.json`, or a later semantic mode run that reads the
events file, finds either the previous complete file or the new complete
file. It never finds a truncated file, or a temporary file left over from
a failed run.

This covers every file a run produces: the compilation database and the
events file.

## Acceptance criteria

- **Given** a successful build, with or without an existing output file,
  **when** Bear writes the output, **then** it writes a temporary file
  and renames it over the output, so a reader never sees a truncated or
  partly written file, and no temporary file remains after the run.
- **Given** a build that runs and fails, **when** Bear finishes, **then**
  the output is replaced with what was captured. A failed build is not an
  error of Bear's own.
- **Given** a run that ends in an error of Bear's own (the build command
  cannot start or be supervised, interception fails, the input cannot be
  read, or writing fails), with or without an existing output file,
  **when** Bear finishes, **then** it reports the error, the previous
  output (if any) is unchanged, and no temporary file is left behind.
  Regression guard for issue #718.
- **Given** an existing database and an events input that semantic mode
  rejects entirely, **when** Bear fails to analyze it, **then** the
  existing database is unchanged and no temporary file is left behind.
- **Given** an output destination that cannot be written, **when** Bear
  starts, **then** it reports the error before the build runs.
- **Given** an output destination that becomes unwritable during the
  build (for example, it is replaced with a non-empty directory), **when**
  Bear renames the temporary file at the end, **then** it reports an IO
  error that names the output path, and leaves the temporary file in
  place for debugging.
- **Given** an existing output file with a name that looks like a
  temporary file (for example `out.tmp`), **when** the run ends in an
  error of Bear's own (for example the build command cannot start),
  **then** that file is unchanged. The temporary file is hidden, sits next
  to the output, and has a unique name, so it never collides with the
  output or with the temporary file of another Bear run.
- **Given** a run that publishes its output, with or without an existing
  output file, **when** Bear publishes it, **then** it has the same
  permissions as a newly created file, not those of the file it replaces.
- **Given** an output path that names an existing file which is not a
  regular file (a device such as `/dev/null`, a named pipe), **when** Bear
  writes the output, **then** it writes to that file in place and does
  not replace it with a regular file. Such an output is not all or
  nothing.

## Known limitations

- Concurrent Bear runs that target the same output file do not corrupt
  it, but the run that finishes last wins.
- The output directory must already exist. Bear does not create missing
  parent directories.
- If Bear is killed by a signal it cannot catch, or cannot stop the
  interception after a failure, the temporary file may be left behind
  (empty or partial).
- When interception fails partway through a build that did start, the
  entries captured before the failure are lost. The previous output
  stays.

## Rationale

- [A failed build still publishes its output](../rationale/failed-build-still-publishes.md)

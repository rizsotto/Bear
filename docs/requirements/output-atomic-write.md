---
title: All-or-nothing output files
status: implemented
---

## Intent

Bear must not leave an output file in a corrupt or partially written state
if the process is interrupted or encounters an error. Users and tools that
consume `compile_commands.json`, or a later `bear semantic` that reads the
events file, must always find either the previous valid version or the new
complete version. They must never find a truncated or half-written file,
or a leftover temporary file from a run that failed.

This covers every file Bear writes as a run's product: the compilation
database and the events file.

## Acceptance criteria

- The output file is written atomically: consumers never see a truncated
  or mid-write file
- Bear replaces the output file only when the run ends without an error
  of its own. A build that runs and fails is not such an error: the
  output holds what was captured
- When the run ends in an error of Bear's own (the build command cannot
  start or be supervised, interception fails, the input cannot be read,
  or writing fails), the previous output file (if any) is unchanged and
  no temporary file is left behind
- A destination that cannot be written is reported before the build runs
- If the final rename fails (e.g. permission denied), the error is reported
  and the temporary file, created next to the final output, is left in
  place for debugging
- The temporary file is hidden and uniquely named, so it never collides
  with the output file (whatever its name) or with the temporary file of
  another Bear run
- The published output file gets the same permissions as a newly created
  file would
- An output path that names an existing file which is not a regular file
  (a device such as `/dev/null`, a named pipe) is written in place and is
  not replaced. Such an output is not all-or-nothing

## Known limitations

- Concurrent Bear runs targeting the same output file do not corrupt it,
  but the run that finishes last wins
- The output directory must already exist; Bear does not create missing
  parent directories
- If Bear itself is killed by a signal it cannot catch, or cannot stop
  the interception after a failure, the temporary file may be left
  behind (empty or partial)

## Testing

Given a successful build, with or without an existing output file:

> When Bear writes `compile_commands.json` (or the events file),
> then a temp file is created during writing
> and renamed to the output file on success,
> atomically replacing any existing file (consumers never see a
> truncated file),
> and the temp file does not exist after completion.

Given a build command that cannot start (e.g. a missing program), with or
without an existing output file:

> When Bear fails to run it,
> then Bear reports the error,
> the output file is unchanged (or still absent),
> and the temp file does not exist.

Given an existing `compile_commands.json` and an events input that Bear
rejects entirely:

> When Bear fails to analyze it,
> then the existing `compile_commands.json` is unchanged
> and the temp file does not exist.

Given a write that fails (e.g. disk full during serialization):

> When writing the new content fails,
> then the original `compile_commands.json` (if any) is unchanged.

Given an output path that names an existing device or named pipe:

> When Bear writes the output,
> then it writes to that file directly
> and does not replace it with a regular file.

Given an existing output file whose name looks like a temp file
(e.g. `out.tmp`) and a build command that cannot start:

> When Bear fails to run it,
> then `out.tmp` is unchanged.

Given an output path that cannot be replaced at the end of the run
(e.g. it became a non-empty directory during the build):

> When Bear attempts to rename the temp file,
> then Bear reports an IO error referencing the output path
> and the temp file remains in place.

## Notes

- The temporary file shares the final output's directory so the rename
  never crosses a filesystem boundary.
- A failing build still publishes: its exit status is the build's result,
  not a failure of Bear, and the entries it did produce are useful.
- When interception fails partway through a build that did start, Bear
  keeps the previous output and drops the partial capture. This is
  provisional; it is to be decided together with the append default.
- GitHub issue #513 originally reported the need for atomic writes after
  users observed corrupted output when Bear/citnames was killed during
  serialization.
- GitHub issue #718 reported the empty temp file left behind when the
  build command could not start.

---
title: Atomic file write for compilation database
status: implemented
---

## Intent

Bear must not leave the compilation database in a corrupt or partially
written state if the process is interrupted or encounters an error during
output. Users and tools that consume `compile_commands.json` must always
find either the previous valid version or the new complete version -- never
a truncated or half-written file.

## Acceptance criteria

- The output file is written atomically: consumers never see a truncated
  or mid-write file
- If writing fails, the previous output file (if any) remains unchanged
- The temporary file is created in the same directory as the final output
- If the final rename fails (e.g. permission denied), the error is reported
  and the temporary file is left in place for debugging
- If writing the new content fails, the temporary file may also be left
  behind (empty or partial)

## Known limitations

- The temp file name is deterministic, so concurrent Bear runs targeting
  the same output file will conflict
- The output directory must already exist; Bear does not create missing
  parent directories

## Testing

Given a successful build:

> When Bear writes `compile_commands.json`,
> then a temp file is created during writing
> and renamed to `compile_commands.json` on success,
> and the temp file does not exist after completion.

Given a successful build with an existing `compile_commands.json`:

> When Bear writes the new output,
> then the old file is atomically replaced
> and consumers never see a truncated file.

Given a write that fails (e.g. disk full during serialization):

> When writing the new content fails,
> then the original `compile_commands.json` (if any) is unchanged
> and the temp file may be left behind (empty or partial).

Given a directory where the user lacks write permission:

> When Bear attempts to rename the temp file,
> then Bear reports an IO error referencing `compile_commands.json`
> (the final path) and the temp file remains in place.

## Notes

- The temporary file shares the final output's directory so the rename
  never crosses a filesystem boundary.
- GitHub issue #513 originally reported the need for atomic writes after
  users observed corrupted output when Bear/citnames was killed during
  serialization.

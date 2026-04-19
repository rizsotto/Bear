---
title: Atomic file write for compilation database
status: implemented
tests:
  - simple_single_file_compilation
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
  (to guarantee same-filesystem rename)
- If the final rename fails (e.g. permission denied), the error is reported
  and the temporary file is left in place for debugging
- If the inner writer fails, the temporary file may also be left behind
  (empty or partial)

## Implementation details

Bear uses the classic temp-file-plus-rename pattern. The temporary file
path is derived from the output path by replacing the extension with `.tmp`
(e.g. `compile_commands.json` becomes `compile_commands.tmp`). This is
deterministic rather than random, which simplifies cleanup but means two
concurrent Bear processes targeting the same output file would race. This
trade-off is intentional: concurrent writes to the same compilation database
are not supported.

The safety guarantee does not depend on signal handling. Because the rename
has not yet occurred during serialization, the original file is not modified
regardless of how the process terminates -- including `SIGKILL`, which
cannot be caught.

### Platform constraints

- **POSIX**: `rename(2)` is atomic when source and destination are on the
  same filesystem. Co-locating the temp file with the output guarantees
  this.
- **Windows**: `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING` provides
  similar guarantees but is not fully atomic under all conditions. Bear
  accepts this platform limitation.

### Error paths

On inner-writer failure (serialization error, disk full), the error
propagates unchanged and references the temp file path. On rename failure,
the error is mapped to reference the final file path. This distinction
helps diagnose whether the problem is serialization or filesystem
permissions.

The atomic write step runs after the append step (`output-append`). The
filtering and serialization stages run inside the atomic writer -- they
write to the temp file, and the atomic writer renames it on success.

## Non-functional constraints

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

> When the inner writer returns an error,
> then the original `compile_commands.json` (if any) is unchanged
> and the temp file may be left behind (empty or partial).

Given a directory where the user lacks write permission:

> When Bear attempts to rename the temp file,
> then Bear reports an IO error referencing `compile_commands.json`
> (the final path) and the temp file remains in place.

## Notes

- GitHub issue #513 originally reported the need for atomic writes after
  users observed corrupted output when Bear/citnames was killed during
  serialization.

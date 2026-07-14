---
title: Path format for file and directory fields
status: implemented
---

## Intent

The JSON compilation database specification
(<https://clang.llvm.org/docs/JSONCompilationDatabase.html>) states that
paths in the `command` or `file` fields must be "either absolute or relative
to [the] directory." Different tools consuming the database have different
expectations: some require absolute paths (clang-tidy historically
segfaulted on relative paths, see LLVM bug #24710), others work better with
relative paths (shorter, portable), and some need canonical paths with
symlinks resolved (clangd struggles with symlinked source trees).

Bear provides configurable path formatting for the `directory`, `file`, and
`output` fields to accommodate these different consumers.

## Acceptance criteria

- The `directory` field path format is configurable
- The `file` field path format is configurable
- The `output` field is formatted using the same strategy as `file`; on
  formatting failure, Bear falls back to the original unformatted path
- Supported path resolution strategies:
  - `as-is` (default) -- preserve the path exactly as observed during
    interception, no transformation applied
  - `absolute` -- convert to an absolute path; does not require the path
    to exist on disk
  - `relative` -- convert to a path relative to the base directory
  - `canonical` -- resolve to the canonical path (resolves symlinks, `.`
    and `..` components); requires the path to exist on disk
- The `directory` field value is the process working directory of the
  intercepted command, formatted according to the chosen strategy using
  itself as the base
- The `file` field is resolved relative to the (already formatted)
  `directory` field
- On Windows, the `canonical` resolver strips the extended-length path
  prefix (`\\?\`) that Windows canonicalization produces, because tools
  like clangd do not understand it (GitHub issue #683)

## Non-functional constraints

- The `canonical` resolver requires the file to exist on disk at the time
  Bear writes the output
- The `absolute` resolver does not require the file to exist (it joins
  the path against the working directory without touching the
  filesystem)
- Path resolution adds minimal overhead for `as-is` (no-op) and `absolute`
  (string manipulation only); the `canonical` resolver performs syscalls
  (`stat`, `readlink`) and is slower on large databases
- Relative path computation handles cross-directory references correctly
  (e.g. `../../other/dir/file.c`)

## Testing

Given a build invoked from `/home/user/project` that compiles `src/main.c`:

> When path format is `as-is` for both directory and file,
> then `directory` is written as-is from the interception,
> and `file` is written as-is (e.g. `src/main.c`).

Given a build where the compiler is invoked with a relative working directory:

> When path format for directory is `absolute`,
> then `directory` is written as an absolute path.

Given a build where the source file is specified as
`/home/user/project/src/main.c`:

> When path format for file is `relative` and directory is
> `/home/user/project`,
> then `file` is written as `src/main.c`.

Given a build where source files are symlinked:

> When path format for file is `canonical`,
> then `file` is written as the resolved real path (symlinks followed).

Given a build on Windows where canonicalize produces `\\?\C:\Users\...`:

> When path format is `canonical`,
> then the `\\?\` prefix is stripped from the output path.

Given a build where the source file does not exist at output time:

> When path format is `canonical`,
> then Bear logs a warning and falls back to the original path for the
> `file` field. If the `directory` cannot be canonicalized, the entire
> entry is dropped with a warning.

Given two directories `/a/b` and `/a/c` with files compiled from each:

> When path format for file is `relative` and directory is `absolute`,
> then files in `/a/b` are relative to `/a/b`,
> files in `/a/c` are relative to `/a/c`,
> and both directory fields are absolute.

Given path format for directory set to `relative`:

> Then the `directory` field resolves to `.` (relative to itself),
> which is valid but rarely useful.

## Notes

- GitHub issue #159 was the original request for absolute paths in the
  output.
- GitHub issue #612 requested canonical/realpath support to work around
  clangd issues with symlinked source trees.
- GitHub issue #683 reported that on Windows/MSYS2, canonical paths include
  the `\\?\` prefix which clangd rejects. The fix strips this prefix after
  canonicalization.
- The `arguments` array paths (include paths, output paths in flags) are
  intentionally not transformed. Transforming them would require a
  compiler-flag-aware path rewriter, which is complex and error-prone.

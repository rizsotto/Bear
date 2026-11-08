# No relative format for `directory`

## Context

The relative path format wrote every entry's `directory` as `.`, the
working directory relative to itself, and configuration validation only
allowed it together with a relative `file`. That combination was never
usable. clangd 22, clang-check, clang-tidy and run-clang-tidy all resolve
a relative `directory` against their own working directory, and they
find an entry by the absolute path of its `file`. An entry whose
`directory` and `file` are both relative is never used, from any working
directory. `.` also threw away where each compilation ran, and the
missing-source filter and header synthesis resolved it from the directory
Bear runs in, so they looked in the wrong place for builds that compile
elsewhere.

LLVM long treated a relative `directory` as unsupported (a 2016 cfe-dev
thread, clangd issue #864 in 2021). PR #235400, merged in November 2026,
resolves it against the folder that holds the database. No release
carries it yet.

The options considered:

- **Keep `.`.** Rejected: the only combination validation allowed is the
  one tools never use.
- **Write `directory` relative to the directory Bear runs in.** Rejected
  for now: released tools still resolve it against their own working
  directory, so it works only for a tool run where Bear ran.
- **Write `directory` relative to the output file's folder,** matching
  PR #235400. Deferred: it depends on unreleased upstream behavior that
  could still change, and on an output location Bear cannot see change
  after the run.
- **Remove the relative format for `directory`.** Chosen. The relative
  format stays for `file`, where it works with an absolute `directory`.

## Decision

The relative format is offered for `file` only. A configuration that
asks for it on `directory` is an error that names the absolute format.

## Consequences

Every format Bear offers gives entries that released clang tools use.
Configurations that set the relative format on `directory` must change
it; they never produced entries those tools used.

Revisit when an LLVM release ships PR #235400. The research and a
proposal for re-adding the format are in the task tracker.

## References

- [output-path-format](../requirements/output-path-format.md), the requirement this supports
- [llvm-project PR #235400](https://github.com/llvm/llvm-project/pull/235400), relative `directory` resolved against the database's folder
- [clangd issue #864](https://github.com/clangd/clangd/issues/864), relative `directory` as an unsupported configuration
- [cfe-dev, February 2016](https://lists.llvm.org/pipermail/cfe-dev/2016-February/047463.html), the earlier discussion

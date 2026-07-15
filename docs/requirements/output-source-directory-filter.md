---
title: Source directory filtering
status: implemented
---

## Intent

Users often want to exclude certain source files from the compilation
database. System headers from `/usr/include`, generated files in `build/`,
or test-only sources in `src/test/` clutter the database and may confuse
downstream tools like clangd or clang-tidy. The source directory filter
lets users define include/exclude rules that control which entries appear
in the output based on the source file path.

## Acceptance criteria

- When no directory rules are configured, all entries are included (no
  filtering)
- When rules are configured, each source file path is evaluated against
  the rule list
- An entry that represents a whole invocation as a single translation
  unit (see `output-compilation-entries`) carries one representative
  source in its `file` field; it is filtered as a unit by that single
  representative path, not per-source. You cannot include half of one
  compilation unit and exclude the other half
- Rules are evaluated in order; the **last** matching rule's action wins
- If no rule matches a source file, the file is **included** by default
- Path matching operates on whole path components, not substrings: a
  rule for `src` matches `src/main.c` but does not match `not_src/main.c`
- A rule matches both files directly in that directory and files in any
  subdirectory (recursive)
- Path matching is case-sensitive when the underlying OS path comparison
  is case-sensitive (always on Unix; filesystem-dependent on Windows)
- No path normalization or canonicalization is performed during matching;
  paths are compared as literal values
- Two actions are supported: `include` and `exclude`
- Empty rule paths are rejected during configuration validation
- Filtered entries are counted in the pipeline statistics

## Non-functional constraints

- No filesystem access is performed during matching (no stat calls, no
  symlink resolution)

## Known limitations

- Symlinks are not resolved during matching. A rule for `/real/path` will
  not match a file accessed via `/symlink/path` even if they point to the
  same location. Users who need symlink-aware filtering should use the
  `canonical` path format (`output-path-format`) so that file paths are
  resolved before matching.
- For a single-translation-unit invocation whose sources straddle an
  included and an excluded directory, only the representative source's
  path is consulted: a rule that matches one of the unit's other sources
  has no effect on whether the unit is kept. Such an input does not occur
  in real builds; the gap is accepted without a dedicated test.

## Testing

Given no directory rules configured:

> When Bear generates the compilation database,
> then all entries are included regardless of file path.

Given a rule that excludes `/usr/include`:

> When a build compiles both `src/main.c` and a file under `/usr/include`,
> then only `src/main.c` appears in the output.

Given rules `include src`, `exclude src/test`, `include src/test/integration`:

> When a build compiles files in all three directories,
> then `src/main.c` is included,
> `src/test/unit.c` is excluded,
> and `src/test/integration/api.c` is included
> (last matching rule wins).

Given an exclude rule for `src/main.c` (exact file path):

> When a build compiles `src/main.c` and `src/main.cpp`,
> then `src/main.c` is excluded
> and `src/main.cpp` is included
> (matching operates on component boundaries, not substrings).

Given a rule for `src`:

> When a build compiles `src/main.c` and `not_src/main.c`,
> then only `src/main.c` matches the rule
> (a rule does not match partial component names).

Given a file path that matches no rule:

> When a build compiles `lib/external.c` and rules only cover `src/`,
> then `lib/external.c` is included (default is include when no rule
> matches).

Given rules with mixed absolute and relative paths:

> When a rule uses `/usr/include` (absolute) and source files use
> relative paths, then the rule does not match those relative paths.
> The user must ensure rule paths match the configured path format
> (`output-path-format`).

Given a build on a case-sensitive filesystem (Unix):

> When a rule excludes `src`,
> then `src/main.c` is excluded
> but `Src/main.c` and `SRC/main.c` are included
> (path comparison follows the OS's case-sensitivity semantics).

Given a rule with an empty path:

> Then configuration validation rejects it with an error.

## Notes

- GitHub issue #261 was the original feature request for include/exclude
  filters on the output.

## Rationale

- [Last matching rule wins](../rationale/source-filter-last-match-wins.md) -
  why precedence is order-based rather than a fixed "exclude wins" policy.

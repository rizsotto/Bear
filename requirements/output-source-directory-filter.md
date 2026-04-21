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
- Rules are evaluated in order; the **last** matching rule's action wins
- If no rule matches a source file, the file is **included** by default
- Path matching uses `Path::starts_with()`, which operates on path
  components (not substrings): a rule for `src` matches `src/main.c` but
  does not match `not_src/main.c`
- A rule matches both files directly in that directory and files in any
  subdirectory (recursive)
- Path matching is case-sensitive when the underlying OS path comparison
  is case-sensitive (always on Unix; filesystem-dependent on Windows)
- No path normalization or canonicalization is performed during matching;
  paths are compared as literal values
- Two actions are supported: `include` and `exclude`
- Empty rule paths are rejected during configuration validation
- Filtered entries are counted in the pipeline statistics

## Implementation details

### Configuration

Source filtering is controlled via the `sources.directories` section of the
configuration file:

```yaml
sources:
  directories:
    - path: src
      action: include
    - path: src/test
      action: exclude
    - path: src/test/integration
      action: include
```

With this configuration:
- `src/main.c` is included (matches rule 1)
- `src/test/unit.c` is excluded (matches rules 1 and 2; rule 2 wins)
- `src/test/integration/api.c` is included (matches rules 1, 2, and 3;
  rule 3 wins)
- `lib/external.c` is included (no rule matches; default is include)

A common pattern is to use `.` as a catch-all rule to include or exclude
everything under the current directory, then add more specific rules after
it.

### Design decisions

**Last-match-wins semantics**: The original feature request (GitHub issue
#261) discussed whether include or exclude should take precedence. The
implementation chose order-based evaluation where the last matching rule
wins. This gives users full control over precedence by ordering their rules
appropriately. It is more flexible than a fixed "exclude always wins"
policy because users can create exceptions to exceptions (as shown in the
example above).

**No path normalization**: Rule paths are matched literally against entry
file paths. For matching to work correctly, rule paths should use the same
format as configured in `format.paths.file` (`output-path-format`). If files are
formatted as absolute paths but rules use relative paths (or vice versa),
matches will not work as expected. This consistency is the user's
responsibility.

**Platform path separators**: Path matching uses `Path::starts_with()`,
which is aware of platform-specific separators (`/` on Unix, `\` on
Windows). Rules must use the appropriate separator for the platform.

The source filter runs before the duplicate filter (`output-duplicate-detection`). This means
entries excluded by source rules are never seen by the duplicate filter.

## Non-functional constraints

- Filtering is a streaming operation with O(r) cost per entry, where r is
  the number of rules
- No filesystem access is performed during matching (no stat calls, no
  symlink resolution)

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
> (`Path::starts_with()` matches on component boundaries, not substrings).

Given a rule for `src`:

> When a build compiles `src/main.c` and `not_src/main.c`,
> then only `src/main.c` matches the rule
> (`Path::starts_with()` does not match partial component names).

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
> (matching delegates to `Path::starts_with()`, which follows OS path
> comparison semantics).

Given a rule with an empty path:

> Then configuration validation rejects it with an error.

## Notes

- GitHub issue #261 was the original feature request for include/exclude
  filters on the output.
- The `only_existing_files` configuration key appeared in older versions of
  Bear but is not implemented in the current Rust codebase. Integration
  tests that reference it in their config YAML rely on serde silently
  ignoring unknown fields.
- Symlinks are not resolved during matching. A rule for `/real/path` will
  not match a file accessed via `/symlink/path` even if they point to the
  same location. Users who need symlink-aware filtering should use the
  `canonical` path format (`output-path-format`) so that file paths are resolved
  before matching.

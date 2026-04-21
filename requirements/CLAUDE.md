## Requirements directory

This directory captures functional and non-functional requirements for Bear.
Requirements are the source of truth for what Bear should do. Tests (integration
and unit) verify that implemented requirements work correctly.

## File naming

```
<area>-<short-name>.md
```

The filename (without extension) is the requirement's unique identifier. Use it
for cross-references in other requirement files and as the value tests cite in
their `Requirements:` tag (see below).

Examples (see existing files in this directory):
- `output-json-compilation-database.md`
- `output-append.md`
- `interception-preload-mechanism.md`

## Requirement template

Every requirement file must follow this structure:

```markdown
---
title: JSON compilation database format
status: implemented
---

## Intent

What the user expects to happen, written from the user's perspective.

## Acceptance criteria

- Criterion 1
- Criterion 2

## Non-functional constraints

Performance, platform support, backwards compatibility, etc.
Only include if relevant.

## Testing

Given-When-Then scenarios that describe how the requirement should be verified.
These are the canonical scenarios; tests implement them.

## Notes

Design decisions, trade-offs, links to issues or discussions.
```

## Status lifecycle

| Status | Meaning |
|---|---|
| `proposed` | Idea captured, not yet reviewed |
| `accepted` | Reviewed, approved for implementation |
| `in-progress` | Implementation started |
| `implemented` | Code complete, tests passing |
| `deferred` | Accepted but postponed (add reason in Notes) |
| `rejected` | Reviewed and declined (add reason in Notes) |

## Linking tests to requirements

Tests cite the requirements they protect using a `Requirements:` tag. The tag
lives in the test source, not in this directory's frontmatter, so renaming or
deleting a test updates the link in the same edit.

Format:

```rust
// Requirements: output-json-compilation-database, output-append
#[test]
fn append_works_as_expected() -> Result<()> { ... }
```

Rules:

- Value is a comma-separated list of requirement IDs (filenames without `.md`).
- Place the tag on the line(s) directly above `#[test]` (or the test macro).
- For a whole file covering a single requirement, use `//! Requirements: <id>`
  at the top of the file. Test-level tags override file-level tags.
- Unit tests in `bear/` and `intercept-preload/` use the same convention.

## Reverse lookup

To find every test that protects a requirement:

```bash
grep -rn "Requirements:.*<requirement-id>" bear/ intercept-preload/ integration-tests/
```

For example, to find tests for `output-append`:

```bash
grep -rn "Requirements:.*output-append" bear/ intercept-preload/ integration-tests/
```

## Coverage check

`requirements/check-coverage.sh` scans every requirement file and verifies that
each `implemented` requirement has at least one `Requirements:` tag referencing
it. Run it from the repo root:

```bash
./requirements/check-coverage.sh
```

The script exits non-zero if any `implemented` requirement lacks coverage.

## How agents should use this

1. **Before implementing a feature**: check if a requirement exists. If not,
   create one with status `proposed` and await approval before coding.
2. **Before modifying behavior**: find the requirement that governs it. Read
   acceptance criteria to understand what must not break.
3. **After implementing**: set status to `implemented` and add a
   `Requirements: <id>` tag to the test(s) that protect the requirement.
4. **When fixing a bug**: check if the bug violates an existing requirement. If
   so, add a test that reproduces the bug and tag it with the requirement ID.

## Incubating new features

Features that are not yet ready for implementation stay at `proposed` or
`accepted`. Use the requirement file to capture:

- User-facing intent (what problem does this solve?)
- Acceptance criteria (how do we know it works?)
- Open questions (what needs to be decided?)

This allows features to mature before code is written. The status field tracks
how far along the feature is. Multiple conversations can incrementally refine
a requirement before it reaches `accepted`.

## Regression protection

The link between requirements and tests is the regression safety net:

- Every `implemented` requirement must have at least one test tagged with its ID
- When a test is renamed or deleted, the tag moves with it (or disappears), so
  the link cannot silently rot
- `check-coverage.sh` catches `implemented` requirements that have drifted to
  zero test coverage

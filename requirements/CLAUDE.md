## Requirements directory

This directory captures functional and non-functional requirements for Bear.
Requirements are the source of truth for what Bear should do. Integration tests
verify that implemented requirements work correctly.

## File naming

```
<area>-<short-name>.md
```

The filename serves as the requirement's unique identifier. Use it for
cross-references in other requirement files (e.g. "see `output-path-format`").

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
tests:
  - test_basic_c_compilation
  - test_output_format_json
---

## Intent

What the user expects to happen, written from the user's perspective.

## Acceptance criteria

- Criterion 1
- Criterion 2

## Non-functional constraints

Performance, platform support, backwards compatibility, etc.
Only include if relevant.

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

## Linking to tests

The `tests:` frontmatter field lists integration test function names that protect
this requirement. When writing a new integration test for a requirement, add the
test name here.

## How agents should use this

1. **Before implementing a feature**: check if a requirement exists. If not, create one
   with status `proposed` and await approval before coding.
2. **Before modifying behavior**: find the requirement that governs it. Read acceptance
   criteria to understand what must not break.
3. **After implementing**: update status to `implemented`, list tests in frontmatter.
4. **When fixing a bug**: check if the bug violates an existing requirement. If so,
   add a test that reproduces the bug and reference it in the requirement.

## Incubating new features

Features that are not yet ready for implementation stay at `proposed` or `accepted`.
Use the requirement file to capture:

- User-facing intent (what problem does this solve?)
- Acceptance criteria (how do we know it works?)
- Open questions (what needs to be decided?)

This allows features to mature before code is written. The status field tracks
how far along the feature is. Multiple conversations can incrementally refine
a requirement before it reaches `accepted`.

## Regression protection

The link between requirements and integration tests is the regression safety net:

- Every `implemented` requirement must have at least one test in its `tests:` field
- If a test is deleted or renamed, update the requirement's `tests:` field
- Run `cargo test` to verify all listed tests still pass

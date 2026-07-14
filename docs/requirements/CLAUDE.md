## Requirements directory

This directory captures functional and non-functional requirements for Bear.
Requirements are the source of truth for what Bear should do. Tests (integration
and unit) verify that implemented requirements work correctly.

Requirements are **contract-only**: they describe what the user can expect, not
where the bits live and not why the design was chosen. Keep literal config keys,
CLI flag names, and schema fragments out of a requirement body - describe the
behaviour ("a configuration option toggles inlining") and let the man page
(`man/bear.1.md`) name the key. The *why* belongs in
[`../rationale/`](../rationale/) (link it from a `## Rationale` section); the
*how* belongs in the code and its comments.

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

## Known limitations

Optional. Gaps in the contract that are accepted rather than planned for
fixing; link the tracking issue when one exists. Omit the section when
there are none.

## Testing

Given-When-Then scenarios that describe how the requirement should be verified.
These are the canonical scenarios; tests implement them.

## Notes

Brief decisions, links to issues or discussions. A one-line decision is fine
here; substantial reasoning or a rejected alternative goes in a rationale entry
instead, linked below.

## Rationale

Optional. A list of links to the rationale entries under
[`../rationale/`](../rationale/) that motivated this requirement - one short
label per link, no prose. Omit the section when there is nothing to link.
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
- Unit tests in `crates/` (e.g. `crates/bear/`, `crates/intercept-preload/`)
  use the same convention.

## Reverse lookup

To find every test that protects a requirement:

```sh
grep -rn "Requirements:.*<requirement-id>" crates/ tests/
```

For example, to find tests for `output-append`:

```sh
grep -rn "Requirements:.*output-append" crates/ tests/
```

## Coverage check

`scripts/check-requirements-coverage.sh` scans every requirement file and
verifies that each `implemented` requirement has at least one `Requirements:`
tag referencing it. Run it from the repo root:

```sh
./scripts/check-requirements-coverage.sh
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

## Things that do NOT belong here

- Literal config keys, CLI flag names, or schema fragments - describe the
  behaviour and let the man page (`man/bear.1.md`) name the key.
- Step-by-step implementation guides, algorithm walkthroughs, error-handling
  tables - those go in code comments next to the implementation.
- Design rationale, trade-offs, or rejected alternatives - those go in
  [`../rationale/`](../rationale/); link them from a `## Rationale` section.
- Bug reports or to-dos - those belong in the issue tracker.

A requirement captures **what the software must do**, not how it is built, not
why we built it that way, and not what we wish it did some day.

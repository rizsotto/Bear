## Requirements directory

This directory captures functional and non-functional requirements for Bear.
Requirements are the source of truth for what Bear should do. Tests (integration
and unit) verify that implemented requirements work correctly.

Requirements are **contract-only**: they describe what the user can expect, not
where the bits live and not why the design was chosen. The *why* belongs in
[`../rationale/`](../rationale/) (link it from a `## Rationale` section); the
*how* belongs in the code and its comments.

### Things that do NOT belong here

- Literal config keys, CLI flag names, or schema fragments - describe the
  behaviour ("a configuration option toggles inlining") and let the man page
  (`man/bear.1.md`) name the key.
- Step-by-step implementation guides, algorithm walkthroughs, error-handling
  tables - those go in code comments next to the implementation.
- Design rationale, trade-offs, or rejected alternatives - those go in
  [`../rationale/`](../rationale/); link them from a `## Rationale` section.
- Bug reports or to-dos - those belong in the issue tracker.

## File naming

```
<area>-<short-name>.md
```

The filename (without extension) is the requirement's unique identifier. Use it
for cross-references in other requirement files and as the value tests cite in
their `Requirements:` tag (see Linking tests to requirements).

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
| `proposed` | Drafted, waiting for the user's review |
| `accepted` | Approved by the user for implementation |
| `implemented` | Code complete, tests passing |
| `rejected` | Declined by the user, or withdrawn (add reason in Notes) |

Only the user moves a requirement out of `proposed`. Agents may create and
edit requirements, but never set `accepted` or `rejected` on their own. When
the user approves or declines a requirement in the current session, the agent
may set the status for them.

## Linking tests to requirements

Tests cite the requirements they protect with a `Requirements:` tag in the
test source. The format, the reverse-lookup grep, and the coverage script
are in [`../../tests/integration/CLAUDE.md`](../../tests/integration/CLAUDE.md);
unit tests in `crates/` use the same convention.

## How agents should use this

1. **Before implementing a feature**: check if a requirement exists. If not,
   create one with status `proposed` and await approval before coding.
2. **Before modifying behavior**: find the requirement that governs it. Read
   acceptance criteria to understand what must not break.
3. **When the contract changes**: edit the requirement first, set its status
   back to `proposed`, and await approval before changing code or tests.
   Git history keeps the previous version.
4. **After implementing**: set status to `implemented` and add a
   `Requirements: <id>` tag to the test(s) that protect the requirement.
5. **When fixing a bug**: check if the bug violates an existing requirement. If
   so, add a test that reproduces the bug and tag it with the requirement ID.

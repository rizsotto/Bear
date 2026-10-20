---
title: <Short title of the contract>
status: proposed
---

<!-- Copy to <area>-<short-name>.md. Status values are in CLAUDE.md. -->

## Intent

What the user expects to happen, from the user's point of view.

## Acceptance criteria

Given-When-Then scenarios. Each one is a single observable outcome that a
test can verify. Use plain rules or a table only where there is no natural
Given, such as a format rule or a list of deliberately excluded names.

- **Given** a build that compiles one source file, **when** Bear finishes,
  **then** the output holds one entry for that file.
- **Given** a build that runs no compiler, **when** Bear finishes, **then**
  the output is an empty JSON array.

A deliberate exclusion that is likely to be proposed again (a name not
recognized, a mode not supported) is a criterion too. Its reasoning goes in
a rationale entry linked below.

## Non-functional constraints

Performance, platform support, backwards compatibility. Omit the section
when there are none.

## Known limitations

Gaps in the contract that are accepted rather than planned for fixing.
Link the tracking issue when one exists. Omit the section when there are
none.

## Rationale

Links to the rationale entries under [`../rationale/`](../rationale/) that
motivated this requirement, one short label per link. A `rejected`
requirement also gives its reason for rejection here, in one line. Omit the
section when it would be empty.

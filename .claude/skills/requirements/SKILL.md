---
name: requirements
description: Write, modify, or review a requirement file under docs/requirements. Pick the single owning file, keep the text contract-only, name IDs so they need no explanation, and verify cross-references and test coverage. Use when the user asks to add, change, audit, or review a requirement, or when the workflow calls for writing one.
---

# Write and maintain requirement files

The rules are in `docs/requirements/CLAUDE.md`; read it first. A new file
fills `docs/requirements/TEMPLATE.md`. This skill is the method. Use it to
write a new requirement, edit one, or audit the directory.

## 1. Find the owner before writing a word

Every behavior has exactly one owning requirement file. Before adding
any passage, find where it belongs:

- `ls docs/requirements/` and grep the directory for the key nouns of
  the behavior. Read every file that hits.
- An owner exists: extend it. No owner: create a new file.
- The new content is one instance of a contract other files also
  instantiate (another recognized toolchain, another output filter):
  extend the shared owner, usually with a table row. Do not create a
  per-instance file. A new file that would restate an existing contract
  with the names changed is a signal to merge, not to add.
- Two files each state half of one contract (one says "exits non-zero
  when X", another says "X never affects the exit code"): that is a
  latent contradiction. Pick or create one owner, state the full
  contract there organized around the principle that resolves the
  halves, and reduce the other files to one-line deferrals.

When another file needs to mention the behavior, it gets a one-line
pointer naming the owner ("owned by `output-duplicate-detection`"),
never a re-narration. If a re-narration can drift, it will.

## 2. Name the ID so it needs no explanation

- Take the prefix from the existing filenames. Do not invent a new
  prefix for one file. Pick the prefix by what the content is about,
  not by which crate implements it.
- Place the new ID next to the most similar existing ID. If a reader
  needs prose to tell them apart, rename until the names alone carry
  the distinction. Disambiguation paragraphs are a naming bug.
- Renaming or merging an ID is one atomic change: update every
  `Requirements:` tag in tests (the lookup grep is in
  `tests/integration/CLAUDE.md`) and every cross-reference in `docs/`.

## 3. Keep the body contract-only

For every sentence ask: is this what the user can expect (keep), why we
chose it (rationale entry), or how it is built (code comment)? Beyond
the list in `docs/requirements/CLAUDE.md`, strike:

- Rust API and type names standing in for behavior
  (`Path::canonicalize`, crate names, function names). State the
  observable behavior instead.
- Internal file names as actors in acceptance criteria (a YAML
  interpreter definition, a module path).
- Buffering or performance speculation dressed as a constraint.
- History: what the code used to do, fixed-bug narratives, unimplemented
  proposals, config keys that never shipped. A regression scenario that
  cites an issue number is contract, not history, and stays.
- Contracts phrased as a diff against the past ("same as before this
  feature"). State the behavior absolutely. An Intent that describes
  the pre-fix bug in present tense turns false once the fix ships.
- Copies of external specifications. Link them.

## 4. Verify every cross-reference, gloss included

A link is two claims: the target exists, and the gloss describes it.
The script below checks the first. For the second, read the target and
confirm the sentence around the link matches what that file owns. A
gloss describing env-derived flags that links the wrapper-mode
mechanism is worse than no link.

## 5. Verify before finishing

```sh
./scripts/check-requirements-coverage.sh
./scripts/check-requirements-references.sh
```

Both must exit 0. The `REVIEW` lines are prompts, not verdicts. A hit in
Intent or acceptance criteria almost always needs a fix. A type named
on purpose as the source of truth for an interchange format stays.

Then read the touched file once more against steps 1 to 4 and
`TEMPLATE.md`.

## Scope of a maintenance edit

When editing an existing file, apply the checks to the passages the
change touches. Fix genre violations you are already rewriting; note
violations elsewhere in the file and propose a separate cleanup rather
than growing the change. A deliberate directory-wide sweep is its own
task with its own commit series.

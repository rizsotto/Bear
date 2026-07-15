---
name: requirements
description: Write, modify, or review a requirement file under docs/requirements -- pick the single owning file, keep the text contract-only, name IDs so they need no explanation, and verify cross-references and test coverage. Use when the user asks to add, change, audit, or review a requirement, or when the decision protocol calls for writing one.
---

# Write and maintain requirement files

`docs/requirements/CLAUDE.md` owns the format: the template, file naming,
status lifecycle, test-tag convention, and the list of what does not
belong. Read it first and follow it; this skill operationalizes it.

This skill adds the method. Every check below traces to a real cleanup
this directory needed: duplicated passages that drifted into apparent
contradiction, implementation detail displacing contract, confusable
names that needed paragraphs to disambiguate, glosses that did not match
their link, and acceptance criteria no test verified. Run the checks
whether writing a new requirement, editing an existing one, or auditing
the directory.

## 1. Read the ground rules

Read `docs/requirements/CLAUDE.md`. If the change involves reasoning or
a rejected alternative, read `docs/rationale/CLAUDE.md` too -- the *why*
goes there, linked from a `## Rationale` section, never inlined.

## 2. Find the owner before writing a word

Every behavior has exactly one owning requirement file. Before adding
any passage, find where it belongs:

- `ls docs/requirements/` and grep the directory for the key nouns of
  the behavior. Read every file that hits.
- An owner exists: extend it. No owner: create a new file.
- The new content is one instance of a contract other files also
  instantiate (another recognized toolchain, another output filter):
  extend the shared owner -- usually a table row -- instead of creating
  a per-instance file. A new file that would restate an existing
  contract with the names changed is a signal to merge, not to add.
- Two files each state half of one contract (one says "exits non-zero
  when X", another says "X never affects the exit code"): that is a
  latent contradiction. Pick or create one owner, state the full
  contract there organized around the principle that resolves the
  halves, and reduce the other files to one-line deferrals.

When another file needs to mention the behavior, it gets a one-line
pointer naming the owner ("owned by `output-duplicate-detection`"),
never a re-narration. If a re-narration can drift, it will.

## 3. Name the ID so it needs no explanation

- Take the prefix from the existing set (`ls docs/requirements/` shows
  them: `cli-`, `interception-`, `output-`, `recognition-`). Do not
  invent a new prefix for one file, and pick the prefix by what the
  content is about, not by which crate implements it.
- Place the new ID next to the most similar existing ID. If a reader
  needs prose to tell them apart, rename until the names alone carry
  the distinction. Disambiguation paragraphs are a naming bug.
- Renaming or merging an ID is one atomic change: update every
  `Requirements:` tag in tests (`grep -rn "Requirements:.*<old-id>"
  crates/ tests/`), every cross-reference in `docs/`, and re-run the
  coverage script.

## 4. Keep the body contract-only

`docs/requirements/CLAUDE.md` ends with the list of what does not
belong; this check is how to apply it. For every sentence ask: is this
what the user can expect (keep), why we chose it (rationale entry), or
how it is built (code comment)? Beyond that list, remove on sight:

- Rust API and type names standing in for behavior
  (`Path::canonicalize`, crate names, function names). State the
  observable behavior instead.
- Internal file names as actors in acceptance criteria (a YAML
  interpreter definition, a module path). The criterion is about
  observable output, not about which file produces it.
- Buffering or performance speculation dressed as a constraint.
- History: what the code used to do, fixed-bug narratives, references
  to unimplemented proposals, config keys that never shipped. A
  requirement states the present contract; git history remembers the
  past. (A regression-guard test scenario citing an issue number is
  fine -- that is contract, not history.)
- Contracts phrased as a diff against the past ("byte-identical to
  today's behavior", "same as before this feature"): state the behavior
  absolutely. And an Intent that describes the pre-fix bug in present
  tense becomes false the day the status turns `implemented`.
- Reproduced external specifications. Link them; do not copy them.

The boundary the no-literal-flags rule needs: it bans flags and config
keys of *Bear itself*; flags of *the compilers Bear classifies* (`-c`,
`--version`, `-showme`) are subject matter and do appear.

## 5. Verify every cross-reference, gloss included

A link is two claims: the target exists, and the gloss describes it.
Check both -- read the target and confirm the sentence around the link
matches what that file actually owns. A gloss describing env-derived
flags that links the wrapper-mode mechanism is worse than no link.

## 6. Record limitations and exclusions where they belong

- Accepted gaps go in the owner file's `## Known limitations` section
  (not in Notes, not in a neighboring file that happens to mention the
  feature); link the tracking issue when one exists.
- A deliberate exclusion likely to be proposed again (a name not
  recognized, a mode not supported) is part of the contract: record it
  in the owner with the reasoning that keeps it out, so the next reader
  does not re-litigate it.

## 7. Make the Testing section carry every criterion

Walk the acceptance criteria one by one; each must map to at least one
Given-When-Then scenario. A criterion with no scenario is either
untestable (rewrite the criterion) or a coverage hole (write the
scenario). Scenarios are canonical; tests implement them and cite the
requirement ID.

Status discipline: a new requirement starts at `proposed` and waits for
approval (see the decision protocol in the root `CLAUDE.md`); the full
lifecycle is the status table in `docs/requirements/CLAUDE.md`. Set
`implemented` only after step 8 passes.

## 8. Verify before finishing

```sh
# every implemented requirement has at least one tagged test
./scripts/check-requirements-coverage.sh

# cross-references resolve (review hits; backticked non-ID words
# sharing a prefix are false positives; keep the prefix list in
# sync with step 3)
grep -hoE '`(cli|interception|output|recognition)-[a-z0-9-]+(\.md)?`' \
    docs/requirements/*.md | tr -d '`' | sed 's/\.md$//' | sort -u | \
    while read -r id; do
      [ -f "docs/requirements/$id.md" ] || echo "dangling: $id"
    done
# IDs are written without the .md extension; a suffixed hit is itself
# a style fix even when the target exists

# implementation-leakage heuristics on the touched file (review hits)
grep -nE '::|\.(rs|yaml|yml)\b|\bcrate\b' docs/requirements/<file>.md
```

The leakage grep is a prompt, not a verdict: hits in Intent or
acceptance criteria almost always need fixing; a type deliberately
named as the source of truth for an interchange format, or test-file
locations in Testing and Notes, are legitimate and stay.

Then a final read of the touched file asking the step questions in
order: single owner (2), self-explaining name (3), contract-only (4),
verified references (5), limitations placed (6), every criterion
tested (7).

## Scope of a maintenance edit

When editing an existing file, apply the checks to the passages the
change touches. Fix genre violations you are already rewriting; note
violations elsewhere in the file and propose a separate cleanup rather
than growing the change. A deliberate directory-wide sweep is its own
task with its own commit series.

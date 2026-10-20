# CLAUDE.md - `site/` guide

This directory is Bear's user-facing documentation site: an
[mdBook](https://rust-lang.github.io/mdBook/) book published to GitHub
Pages at `https://rizsotto.github.io/Bear/`. The deploy job runs from
`master` only. The site is for users; `docs/` (requirements and
rationale) is for contributors, and the two do not overlap.

The site follows [Diataxis](https://diataxis.fr/): every page except the
three landing pages is one of four types (tutorial, how-to, reference,
explanation), and the rules below apply it.

## What lives where

| Path | Holds |
|---|---|
| `book.toml` | Book configuration. Title "Bear"; `site-url = "/Bear/"`. |
| `src/SUMMARY.md` | The table of contents. Every page must be listed. |
| `src/index.md`, `src/installation.md` | The only pages at the root: the home page, and installing, which sits above the sections. |
| `src/404.md` | The not-found page. Deliberately NOT in `SUMMARY.md` (listing it would put it in the navigation). Its links are absolute (`/Bear/...`), because GitHub Pages serves it for any missing path. Keep them in step with `site-url` and the directories below. |
| `src/tutorials/` | The tutorial pages. |
| `src/guides/` | `troubleshooting.md`, plus `recipes/` below it. |
| `src/guides/recipes/` | Task pages plus `index.md`, the recipe index. |
| `src/reference/` | `command-line.md`, `configuration.md`, `exit-status.md` and `supported-compilers.md`. `command-line.md` and `supported-compilers.md` are partly generated: only the block between the `BEGIN GENERATED` and `END GENERATED` markers comes from the script; the prose outside it is edited here. |
| `src/understanding/` | The explanation pages. |
| `src/platforms/` | One page per operating system. |
| `theme/head.hbs` | Site-wide static `<head>` content: the Google Search Console token and nothing else. Removing it un-verifies the property. |
| `book/` | Build output. Git-ignored, never edited. |

A page's folder matches its `SUMMARY.md` section and therefore its
Diataxis type. Moving a page between sections means moving its file too.

## Authoritative sources - never document from memory

- **Command-line flags, configuration keys, and their defaults**: check
  them against `man/bear.1.md` or `bear --help`. Pages link the site's
  `src/reference/` pages, not the man page.
- **Recognized compiler and launcher names**:
  `build-support/compilers-codegen/tests/snapshots/snapshots__snapshot_recognition.snap`,
  or `bear semantic --print-compilers` for the live form. Do not copy
  such a list from the wiki, an old release note, or memory.
- **Behaviour of a command**: a scratch install of a debug build, or a
  named integration test under `tests/integration/`. There is no
  `target/debug/bear`; `bear` is a shell wrapper that
  `scripts/install.sh` generates with the install prefix baked in:

  ```sh
  cargo build
  SRCDIR=target/debug PREFIX=/tmp/bear-review INTERCEPT_LIBDIR=lib \
      ./scripts/install.sh
  /tmp/bear-review/bin/bear --version
  ```

## Content rules

1. **One page per search intent, not per feature.** If a page answers
   two questions, split it.
2. **The H1 and the chapter name are phrased as the user asks the
   question**: "Generate compile_commands.json for a Makefile project",
   not "Combined mode". The `SUMMARY.md` entry becomes the HTML title,
   so keep it and the H1 in agreement.
3. **The first paragraph gives the working answer**, normally the
   command to run. Background and mechanism come after.
   `scripts/postprocess-docs-site.sh` lifts this paragraph as the page's
   `<meta name="description">` (about 155 characters), so make it a
   real, self-contained sentence.
4. **Every shell command shown must be verified.** Run it against a
   debug build, or cite the integration test that exercises it. The page
   never says so: no "Verified:", no "tested against", no note about what
   could not be checked. State the behaviour as fact, or leave it out.
   Put the evidence in the commit message or pull request.
5. **Wiki-migrated text is a source, not a truth.** Verify each claim
   against current 4.x behaviour.
6. **ASCII only**, as root `CLAUDE.md` ("Output format") says.
7. **Kebab-case file names that match the query**, for example
   `compile-commands-for-makefile.md`.
8. **Cross-link related pages, and link back to the recipe index.**
   Every recipe links to `recipes/index.md`, and `recipes/index.md`
   lists every recipe in the same order as `SUMMARY.md`. The check
   script only verifies `SUMMARY.md`.
9. **Exactly one Diataxis type per page, declared and obeyed.** The
   first line of every source file names its type:

   ```markdown
   <!-- Diataxis type: how-to -->
   ```

   A how-to does not explain mechanism; it links to `how-it-works.md`.
   An explanation page carries no commands to copy; it links to the
   recipe.
10. **A page states facts about Bear, never facts about itself.** No
    roadmap of the headings ("The rest of this page covers ..."), no
    statement of what the page is for, no description of its relation to
    another document, no defending a writing decision. The rules in this
    file are for the writer; do not restate one on a page. Two forms stay,
    because they send the reader somewhere:

    - a tutorial's "By the end of this page you will have ..." opening;
    - a scope boundary that routes, as `troubleshooting.md` opens by
      pointing at the empty-database recipe.

    The test is direction. A sentence that sends the reader somewhere
    stays; a sentence that describes where they already are goes.

## Code-block tags

mdBook's bundled highlight.js picks a grammar from the fence tag, so the
tag matters:

1. **Commands with no output shown**: ```` ```sh ````. No `$` prompt,
   so the block stays copyable.
2. **A command together with its output**: a SINGLE fence tagged
   ```` ```shell ````. Prefix every command line with `$ ` and leave
   output lines unprefixed. Do not use `shell` on a block with no prompt.
3. **Output alone, or a file's contents**: a bare fence, no tag.

`yaml`, `json`, `dockerfile`, and similar name their own grammar.

## Page types

| Type | Purpose | Where |
|---|---|---|
| tutorial | One narrow path from start to finish, for a first-timer. Few of them. | `src/tutorials/` |
| how-to | The paths a reader can take through a task, dwelling only where attention is needed. | `src/guides/`, `src/platforms/` |
| reference | Enumeration, looked up rather than read. Must be exactly correct. | `src/reference/` |
| explanation | Mechanism and design. No commands. | `src/understanding/` |

Prefer generating a new reference page over writing one.

`src/index.md`, `src/guides/recipes/index.md` and `src/404.md` are the
only files allowed to declare `<!-- Diataxis type: landing (navigation
page, not one of the four types) -->`. Do not add a fourth.

### What a how-to owes the reader

- **Cover the paths, not one path.** Admit the branches the reader's
  situation can take, and say which one they are on.
- **Dwell only where attention is needed.** The parts that just work get
  a sentence; spend the page on the step that fails, surprises, or needs
  a decision.
- **Point at reference, constantly.** Every flag, key, compiler name and
  exit status has one home under `src/reference/`. Link it; do not
  restate it.
- **Do not send the reader to troubleshooting instead of reference.**

A how-to with nothing left once the reference facts are stripped is not
a how-to. Merge it into a sibling, or give it the failure mode that
justifies it.

## Navigation sections

A new page goes in the `SUMMARY.md` section that matches its type:

| Section | Holds |
|---|---|
| (ungrouped, first) | `installation.md` |
| Tutorials | The tutorial pages. |
| Guides | How-to pages: the recipes, and troubleshooting. |
| Reference | The `src/reference/` pages. |
| Understanding Bear | The explanation pages. |
| Platform notes | How-to pages shelved by operating system. |

Do not rename the sections. Why they are named this way is in
[`../docs/rationale/docs-site-over-wiki.md`](../docs/rationale/docs-site-over-wiki.md).

## Before committing

Run `./scripts/check-docs-site.sh`. It must print `OK`; its header lists
what it checks. CI runs it from `.github/workflows/pages.yml`, which pins
the mdBook version to install locally. Two habits it depends on:

- Write intra-book links inline, `[text](../recipes/index.md)`, and keep
  reference-style definitions for external URLs. The link scan reads
  inline links only, and it also reads fenced code, so an
  example link to a missing page needs a non-`.md` name.
- Regenerate the generated reference pages with their source change:
  `scripts/generate-supported-compilers.py` after a compiler YAML edit,
  `scripts/generate-command-line-reference.py` after a CLI change (needs
  the scratch install above).

Do not add mdBook plugins.

## Writing checklist

- The page is listed in `SUMMARY.md`, and a recipe on `recipes/index.md` too.
- The type comment is on the first line and the page stays inside it.
- The first paragraph answers the question.
- No sentence describes the page itself.
- Every command was run, or an integration test is cited.
- Every flag, key, and default was checked against `man/bear.1.md`.
- Every compiler name was checked against the recognition snapshot.
- Links to related pages, and back to the recipe index.

# CLAUDE.md - `site/` guide

This directory is Bear's user-facing documentation site: an
[mdBook](https://rust-lang.github.io/mdBook/) book that is published to
GitHub Pages at `https://rizsotto.github.io/Bear/`. It is the canonical
user documentation and it replaces the project wiki, whose pages are
being reduced to one-line pointers here. CI checks the build on every
push and pull request; the deploy job runs from `master` only, so
nothing goes live from a topic branch.

This file is the complete set of rules for writing pages here. You do
not need to read any other document to use it.

## What lives where

| Path | Holds |
|---|---|
| `book.toml` | Book configuration. Title "Bear"; `site-url = "/Bear/"`. |
| `src/SUMMARY.md` | The table of contents. Every page must be listed. |
| `src/index.md`, `src/installation.md` | The only pages at the root: the home page, and installing, which sits above the sections. |
| `src/404.md` | The not-found page. Deliberately NOT in `SUMMARY.md`: mdBook renders it specially, and listing it would put it in the navigation. Its links are absolute (`/Bear/...`) because GitHub Pages serves it for any missing path, including deep ones, and relative links would resolve against that path. Keep them in step with `site-url` in `book.toml`, and with the directories below. |
| `src/tutorials/` | The tutorial pages. |
| `src/guides/` | `troubleshooting.md`, plus `recipes/` below it. |
| `src/guides/recipes/` | Task pages plus `index.md`, the recipe index. |
| `src/reference/` | `configuration.md` and `supported-compilers.md`. |
| `src/reference/supported-compilers.md` | Partly generated. `scripts/generate-supported-compilers.py` renders the compiler-family tables from `crates/bear/compilers/*.yaml` into the block between its `<!-- BEGIN GENERATED -->` and `<!-- END GENERATED -->` markers. Everything outside those markers is hand-written prose, edited here directly. |
| `src/understanding/` | The explanation pages. |
| `src/platforms/` | One page per operating system. |

The directory layout mirrors the `SUMMARY.md` sections, so a page's
folder tells you which section it belongs to and therefore which Diataxis
type it must declare. A new page goes in the folder for its type; moving
a page between sections means moving its file too.
| `book/` | Build output. Generated, git-ignored, never edited. |

The site does not overlap with `docs/`, which holds requirements
(contracts) and rationale (decision records) for contributors. The site
is for users.

## Authoritative sources - never document from memory

- **Command-line flags, configuration keys, and their defaults**:
  `man/bear.1.md`. It is the reference; the site does not duplicate it,
  it links to it. Every flag or key that appears on a site page must
  exist there or in `bear --help` output.
- **Recognized compiler and launcher executable names**:
  `build-support/compilers-codegen/tests/snapshots/snapshots__snapshot_recognition.snap`.
  This snapshot is generated from the compiler definitions, so it is the
  only correct source for toolchain pages. Do not copy such a list from
  the wiki, from an old release note, or from memory.
- **Behaviour of a command**: a scratch install of a debug build, or a
  named integration test under `tests/integration/`. There is no
  `target/debug/bear` binary to run: `bear` is a shell wrapper that
  `scripts/install.sh` generates around `bear-driver`, and it embeds
  the install prefix as a literal path. Build, then install to a
  throwaway prefix and run that copy:

  ```sh
  cargo build
  SRCDIR=target/debug PREFIX=/tmp/bear-review INTERCEPT_LIBDIR=lib \
      ./scripts/install.sh
  /tmp/bear-review/bin/bear --version
  ```

  `bear semantic --print-compilers` prints every recognized compiler
  name with the `as:` value it maps to, which is the live form of the
  recognition snapshot above.

## Content rules

1. **One page per search intent, not per feature.** A page exists to
   answer one question a user types into a search engine. If a page
   answers two, split it.
2. **The H1 and the chapter name are phrased as the user asks the
   question**, not as the codebase names the feature: "Generate
   compile_commands.json for a Makefile project", not "Combined mode".
   mdBook renders a page title as "<chapter name> - <book title>", so
   the chapter name in `SUMMARY.md` is the HTML title element. Keep the
   `SUMMARY.md` entry and the H1 in agreement.
3. **The first paragraph gives the working answer**, normally the
   command to run. Background, caveats, and mechanism come after. This
   is what earns search snippets and citations from AI assistants, and
   it is the rule most easily lost by opening with two paragraphs about
   `LD_PRELOAD`.
4. **Every shell command shown must be verified.** Run it against a
   debug build, or cite the integration test that exercises it. An
   unverified command does not ship. Verification is a precondition for
   writing a claim, never a thing the page tells the reader about: no
   "Verified:", no "tested against", and no note about what could not be
   checked while writing. State the behaviour as fact, or leave it out.
   A page that hedges its own commands teaches the reader to distrust
   all of them. Put the evidence in the commit message or the pull
   request, where it belongs.
5. **Wiki-migrated text is a source, not a truth.** Verify each claim
   against current 4.x behaviour before it lands. The wiki carries
   3.x-era statements that are simply wrong now.
6. **ASCII only.** No em dashes, smart quotes, or Unicode bullets. Use
   hyphens, straight quotes, three dots, and `-` or `*` for bullets.
7. **Kebab-case file names that match the query**, for example
   `compile-commands-for-makefile.md`.
8. **Cross-link related pages, and link back to the recipe index.**
   Every recipe links to `recipes/index.md`; other pages link to the
   recipes, platform pages, or explanation pages a reader needs next.
9. **Exactly one Diataxis type per page, declared and obeyed.** The
   first line of every source file is an HTML comment naming its type:

   ```markdown
   <!-- Diataxis type: how-to -->
   ```

   The page then stays inside that type. A how-to does not explain
   mechanism; it links to `how-it-works.md`. An explanation page carries
   no commands to copy; it links to the recipe. This rule is what keeps
   rule 3 honest.

## Code-block tags

mdBook 0.5.4's bundled highlight.js resolves a fence's tag to a specific
grammar, so the tag is a functional choice, not decoration. Three cases:

1. **Commands with no output shown** are tagged ```` ```sh ````. It is the
   Bash grammar: it highlights quoted strings, variables, comments, and
   builtins, and it carries no `$` prompt, so the block stays copyable as
   a unit. This is the common case: one or more commands the reader runs,
   nothing else.
2. **A command together with the output it produced** is a transcript,
   not something to copy-paste, so it becomes a SINGLE fence tagged
   ```` ```shell ````, the "Shell Session" grammar. Its only rule
   highlights a leading `$`, `#`, `>`, or `%` prompt, so prefix every
   command line with `$ ` and leave the output lines that follow it
   unprefixed. Do not use `shell` on a block with no prompt in it.
3. **Output alone, or a file's contents, with no command** stays a bare
   fence: a JSON entry, a log excerpt, an error message shown as a
   symptom. No language means no highlighting, which is correct here
   since there is nothing to highlight.

`yaml`, `json`, `dockerfile`, and similar are unaffected by this; they
name their own grammar already.

## The page-type map

| Type | Purpose | Pages |
|---|---|---|
| tutorial | The first successful run; learning by doing. | `getting-started.md` |
| how-to | One task, for a reader who already knows what they want. | `recipes/*`, platform pages, troubleshooting |
| reference | Enumeration; looked up, not read. | `man/bear.1.md` owns flags, configuration keys, and defaults, and the site does not duplicate it. `supported-compilers.md` is the site's one reference page: it enumerates recognized compilers, and it is generated from the compiler definitions rather than written, so it cannot drift from them. Do not add a hand-written reference page. |
| explanation | Mechanism and design. No commands. | `how-it-works.md` |

Pages outside that table: `installation.md` is a how-to;
`configuration.md` and `faq.md` are explanation. `configuration.md`
explains what the sections are for and states each one's default, and
sends the reader to the man page for the exact keys; stating a default is
explanation, but a copy-paste configuration recipe is not, and does not
belong there.
An FAQ answer that grows into a task with commands moves to its own
recipe.

Three pages are navigation rather than content: `src/index.md` (the home
page), `src/recipes/index.md`, and `src/404.md`. They are the only files
allowed to declare `<!-- Diataxis type: landing (navigation page, not one
of the four types) -->`. Do not add a fourth.

## Navigation separates the types, without naming them Diataxis's way

`SUMMARY.md` has these sections, and a new page belongs in the one that
matches its declared type:

| Section | Holds |
|---|---|
| (ungrouped, first) | `installation.md`, which precedes anything a reader can do. |
| Tutorials | The tutorial pages. |
| Guides | How-to pages: the recipes, and troubleshooting. |
| Reference | `configuration.md` and `supported-compilers.md`. |
| Understanding Bear | The explanation pages. |
| Platform notes | How-to pages shelved by operating system instead. |

Do not rename "Guides" to "How-to guides" or "Understanding Bear" to
"Explanation". Those two Diataxis names describe a document to its
author; "Tutorials" and "Reference" are words a reader applies to
themselves. Section names are also load-bearing for search, because
mdBook renders a page title as "<chapter name> - <book title>". And do
not leave a page in a section that does not match its type comment: a
reference page among the how-tos is a mismatch a reader notices. The
reasoning, including what this decision used to say, is in
[`../docs/rationale/docs-site-over-wiki.md`](../docs/rationale/docs-site-over-wiki.md).

## The build-and-link check

Before committing any change under `site/`, run:

```sh
./scripts/check-docs-site.sh
```

It must print `OK` and exit 0. The check has five parts:

1. `mdbook build site` must succeed and print no `WARN` or `ERROR` log
   line. Warnings are fatal on purpose: mdBook reports preprocessor and
   include failures that way, and they would otherwise ship silently. A
   chapter listed in `SUMMARY.md` whose file is missing is a hard error,
   because `book.toml` sets `build.create-missing = false` (the default
   would quietly create an empty page instead).
2. Every inline Markdown link to a `.md` target must resolve to an
   existing file. mdBook does not check this itself: verified against
   mdBook 0.5.4, a link to a nonexistent page builds clean. A link title
   and an `#anchor` are both allowed and are stripped before the lookup.
   Because the scan looks at inline links only, write intra-book links
   inline, `[text](../recipes/index.md)`, and keep reference-style
   definitions for external URLs. The scan is textual and does not skip
   fenced code blocks, so an example link to a file that is meant not to
   exist will be reported as broken; use a non-`.md` name in such an
   example.
3. Every page under `src/` must be listed in `SUMMARY.md`. mdBook
   silently ignores files that are not listed, so such a page would
   never be served. `404.md` is the one exception, for the reason in the
   table above.
4. Every absolute `/Bear/<page>.html` link in `404.md` must have a
   matching `<page>.md`. Part 2 only follows `.md` targets, so without
   this the not-found page could ship a dead link.
5. `src/supported-compilers.md` must match what
   `scripts/generate-supported-compilers.py` produces from the current
   `crates/bear/compilers/*.yaml`. After changing a compiler YAML file,
   or the generator, run `python3
   scripts/generate-supported-compilers.py` and commit its output with
   the YAML change. Commenting an entry out does not count as listing
   it, and `(./page.md)` counts the same as `(page.md)`.

`.github/workflows/pages.yml` runs the same script on every push and
pull request, and it is the single source of truth for the pinned mdBook
version. Install that same version locally
(`cargo install mdbook --version <pinned> --locked`).

There are no mdBook plugins, including `mdbook-linkcheck`. Dependency
minimalism applies to documentation tooling too. Do not add one.

## Writing checklist

- The page is listed in `SUMMARY.md` and the file exists.
- The type comment is on the first line and the page stays inside it.
- The first paragraph answers the question.
- Every command was run, or an integration test is cited.
- Every flag, key, and default was checked against `man/bear.1.md`.
- Every compiler name was checked against the recognition snapshot.
- ASCII only; the repository-wide `codespell` job also reads these
  files.
- Links to related pages, and back to the recipe index.

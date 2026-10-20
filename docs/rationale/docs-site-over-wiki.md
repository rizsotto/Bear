# A documentation site instead of the wiki

## Context

Bear's user documentation lived in the
[GitHub wiki](https://github.com/rizsotto/Bear/wiki): 14 pages on
installing, usage, configuration, platforms, troubleshooting, and an FAQ.
The problem it did not solve is discovery. The query a likely user types
("generate compile_commands.json for a Makefile project") did not
surface Bear at all.

The wiki cannot fix that. Checked on 2026-07-06: wiki pages can be
crawled (no `robots.txt` block, no `noindex`), but they cannot rank. The
wiki gives no control over the HTML title, has no sitemap, cannot be a
Google Search Console property, and builds no link authority of its own.
It is also outside the pull-request flow, so it drifts from the code
with nothing in CI to catch it. It still had 3.x-era text years after
4.x shipped.

The forces on the replacement: the toolchain is Rust and CI already has
cargo; the minimal-dependency rule applies to doc tooling too; the set
of pages was small (about 15); and the wiki has inbound links from
issues, forums and other projects' docs that must not start to 404.

Options considered:

- Expand the wiki. Cheapest to write, but it cannot be made to rank,
  and it stays outside code review.
- In-repo Markdown only, no site. Free and reviewed, but
  `github.com/robots.txt` disallows `/*/tree/`, so search engines do not
  index those pages at all.
- mkdocs-material. Better search and SEO features out of the box than
  mdBook, but it brings a Python toolchain and a dependency tree into a
  project that has neither. Not worth it for about 15 pages.
- [mdBook](https://rust-lang.github.io/mdBook/), with no plugins.
  Chosen: one static binary from the Rust ecosystem, pinned in CI.

For structure, the four [Diataxis](https://diataxis.fr/) types
(tutorial, how-to, reference, explanation) were adopted. First the
contents were organized purely by task, with the types only an authoring
rule. That failed as pages grew: a reference page sat among how-tos and
an explanation page sat next to a recipe, and readers could see the
mismatch. So the types now separate the sections. Diataxis's own section
names were rejected: "How-to guides" and "Explanation" are words for
authors, not readers. Nobody arrives wanting an "Explanation". Section
names also matter for search, because mdBook renders a page title as
"<chapter name> - <book title>".

## Decision

The user documentation is an mdBook book built from `site/` and served
from GitHub Pages at <https://rizsotto.github.io/Bear/>. Each page is
one Diataxis type, and the sections are named "Tutorials", "Guides",
"Reference" and "Understanding Bear", plus "Platform notes". The wiki
stays, with each page cut to a pointer to its replacement.

Installing sits above the sections, ungrouped, because it comes before
anything a reader can do with the tool.

## Consequences

- Pages are reviewed with the code in pull requests. Every claim can be
  checked against the man page, the compiler snapshot, or a debug build
  before it ships.
- The site controls its titles, sitemap and canonical URLs, and is a
  Search Console property. This makes ranking possible; it does not make
  it happen. No lift is assumed.
- Documentation is a build step. A broken internal link or a page
  missing from `SUMMARY.md` fails CI. That cost is intended.
- Wiki page titles stay, so old inbound links still land somewhere
  useful. Content is never forked between the wiki and the site.
- One type per page forces splits a wiki page used to absorb. The old
  Usage page was a tutorial, several how-tos and a flag reference at
  once.
- The reference section is generated where it can be: the command-line
  page from the binary's help, the compiler list from the compiler
  definitions. That is the only way a second copy of a contract stays
  true. `man/bear.1.md` stays the offline copy.

## References

- [site/CLAUDE.md](../../site/CLAUDE.md), the authoring rules for the site
- [structured-data-scope](structured-data-scope.md), what the site emits for search engines
- [Diataxis](https://diataxis.fr/), the four documentation types
- [mdBook](https://rust-lang.github.io/mdBook/), the site generator
- [Bear wiki](https://github.com/rizsotto/Bear/wiki), the old location

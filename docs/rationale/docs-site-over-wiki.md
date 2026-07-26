# A documentation site instead of the wiki

## Context

Bear's user documentation lived in the GitHub wiki: 14 pages covering
installation, usage, configuration, platform notes, troubleshooting, and
an FAQ. The problem it does not solve is discovery. The canonical query
a potential user types ("generate compile_commands.json for a Makefile
project") does not surface Bear at all, so people who need the tool do
not find it.

The wiki cannot fix that. Verified 2026-07-06: wiki pages are crawlable
(no `robots.txt` block, no `noindex` meta tag), but they are
structurally unrankable. The wiki gives no control over the HTML title
element, publishes no sitemap, cannot be registered as a property in
Google Search Console, and accumulates no link authority of its own.
Wiki content is also outside the pull-request workflow, so it drifts
from the code with nothing in CI to catch it: the wiki still carried
3.x-era text years after 4.x shipped.

Serving the same Markdown from the repository is not a fix either.
`github.com/robots.txt` disallows `/*/tree/` paths, so files rendered in
the repository browser are not indexed at all.

The forces on the replacement: the project's toolchain is Rust and the
CI images already have cargo; the repository keeps dependencies minimal
and that rule applies to documentation tooling too; the estate is small
(about 15 pages); and the wiki has inbound links from issues, forum
posts, and third-party documentation that must not start returning 404.

## Decision

User documentation is an [mdBook](https://rust-lang.github.io/mdBook/)
book built from `site/` in this repository and deployed to GitHub Pages
at `https://rizsotto.github.io/Bear/`. That site is the canonical user
documentation. mdBook is a single static binary from the Rust ecosystem
with no runtime dependencies, installed pinned by version in CI, and no
plugins are used.

The wiki stays enabled. Each page is reduced to a one-line pointer to
its replacement on the site, keeping the page titles so existing inbound
links still resolve.

The four documentation types of [Diataxis](https://diataxis.fr/)
(tutorial, how-to, reference, explanation) are adopted as a per-page
authoring discipline: every source page declares exactly one type in an
HTML comment at the top and stays inside it.

The table of contents separates those types too, but it does not use
Diataxis's own names for them. The sections are "Tutorials", "Guides",
"Reference", and "Understanding Bear", plus a topical "Platform notes"
group that holds how-to pages shelved by operating system instead. The
naming is the whole point: "Tutorials" and "Reference" are words readers
already apply to themselves, while "How-to guides" and "Explanation"
name kinds of document to their authors. A reader who wants to look
something up recognizes "Reference"; nobody arrives wanting an
"Explanation".

Installing sits above the sections, ungrouped, because it precedes
anything a reader can do with the tool.

## Consequences

- Pages are reviewed with the code, in pull requests, and every claim
  can be checked against the man page, the compiler-recognition
  snapshot, or a debug build before it ships.
- The site controls its own titles, sitemap, and canonical URLs, and can
  be registered in Search Console. This makes ranking possible; it does
  not make it happen. No lift is assumed.
- Documentation is now a build step. A broken intra-book link or a page
  missing from `SUMMARY.md` fails CI, which is the intended cost.
- Two places to keep honest during the transition, until the wiki pages
  are stubbed down to pointers. Content must never be forked between
  them.
- The one-page-per-type discipline forces splits that a single wiki page
  used to absorb. The old Usage page was a tutorial, several how-tos,
  and a flag reference in one document, and it becomes several pages.
- The reference type has no page on the site at all: `man/bear.1.md`
  owns flags, configuration keys, and defaults, and the site links to it
  rather than restating it.

Rejected alternatives:

- **Expand the wiki.** Cheapest to write, but it cannot be made
  rankable (no title control, no sitemap, no Search Console property),
  and it keeps documentation outside code review.
- **mkdocs-material.** Better out-of-the-box search and SEO features
  than mdBook, at the cost of a Python toolchain and a dependency tree
  in a project that has neither. Not worth it for about 15 pages.
- **In-repo Markdown only, no site.** Free, and it does put the text
  under review, but `github.com/robots.txt` disallows `/*/tree/`, so it
  cannot be found by search at all.
- **Diataxis's own section names in the navigation**, that is,
  `SUMMARY.md` sections headed "How-to guides" and "Explanation". The
  types are what the sections separate, but two of the four names are
  authoring vocabulary, so they are shelved under "Guides" and
  "Understanding Bear" instead. Section names are also load-bearing for
  search, because mdBook renders a page title as "<chapter name> - <book
  title>".

  This entry originally recorded the stronger decision that the four
  types would not shape the navigation at all, and that the contents
  would be organized purely by task. That did not survive contact with
  the estate: as pages accumulated, a reference page sat in a section of
  how-tos and an explanation page sat next to a recipe, and the mismatch
  was visible to readers rather than only to authors. The types now
  separate the sections, under the names above. What was rejected is the
  vocabulary, not the taxonomy.

## References

- Authoring rules for the site: `site/CLAUDE.md`
- https://diataxis.fr/ - the four documentation types
- https://rust-lang.github.io/mdBook/ - the site generator
- https://github.com/rizsotto/Bear/wiki - the superseded location

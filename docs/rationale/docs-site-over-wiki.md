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
HTML comment at the top and stays inside it. They are rejected as a
navigation scheme: the table of contents is organized by what a reader
is trying to do, not by document type. One exception is deliberate. The
tutorials sit under a "Tutorials" heading, because that is the one type
name a reader applies to themselves rather than to a document: someone
new to the tool is looking for a tutorial and knows it. "How-to guides",
"Reference", and "Explanation" are labels for authors, and the sections
they would create are shelved by task instead.

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
- **Diataxis as the full navigation scheme**, that is, `SUMMARY.md`
  sections named "Tutorials", "How-to guides", "Reference",
  "Explanation". Three of those four put authoring jargon in front of
  readers who came looking for one answer, and some would hold a single
  page. It also dilutes the query-shaped chapter names, because mdBook
  renders a page title as "<chapter name> - <book title>", which makes
  the chapter name the title element that search engines read. The
  visible-taxonomy form of Diataxis is aimed at estates far larger than
  this one. The discipline is kept, and of the shelving only the
  "Tutorials" heading, for the reason given under Decision.

## References

- Authoring rules for the site: `site/CLAUDE.md`
- https://diataxis.fr/ - the four documentation types
- https://rust-lang.github.io/mdBook/ - the site generator
- https://github.com/rizsotto/Bear/wiki - the superseded location

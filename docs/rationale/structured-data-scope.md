# Structured data: what the site emits, and what it deliberately does not

## Context

The documentation site build injects JSON-LD structured data into every
rendered page (see `scripts/postprocess-docs-site.sh`). Before choosing
which schema.org types to emit, the obvious candidates were checked
against current Google Search behavior, because the schema.org
vocabulary being valid says nothing about whether Google still rewards
it. Source:
https://developers.google.com/search/docs/appearance/structured-data/search-gallery

Three types looked tempting for this site and were rejected:

- **`HowTo`**. The recipe pages (`site/src/recipes/*.md`) are
  numbered-steps task pages, structurally a textbook `HowTo`. But Google
  removed `HowTo` rich results from mobile search in August 2023 and
  from desktop in September 2023. The markup would render nothing.
- **`FAQPage`**. `site/src/faq.md` is structurally a textbook `FAQPage`.
  Google restricted `FAQPage` rich results to government and health
  sites in August 2023, then fully deprecated the feature on 2026-05-07:
  Search Console reporting for it ends June 2026, and API support ends
  August 2026. The markup would render nothing, and the reporting that
  would have shown that is itself going away.
- **`WebSite` with a `SearchAction`** (the sitelinks searchbox that used
  to appear under a site's result on a branded query). Google retired
  this rich result in November 2024. Bear's site is not the kind of
  destination that would plausibly have earned a sitelinks searchbox
  anyway, but the type is a common default addition and worth naming as
  rejected before someone adds it out of habit.

All three remain valid schema.org types, and Google still parses them
without penalty; they simply produce no search-result lift any more.
Emitting them would be dead weight: markup to maintain, for a rich
result that no longer exists. The recipe pages and `faq.md` are the
obvious but wrong candidates for `HowTo` and `FAQPage` respectively;
without this record, someone will look at those pages later, notice the
structural match, and re-propose exactly the markup this entry rejects.

## Decision

The site emits only two JSON-LD types, both still in Google's supported
structured-data gallery as of this writing:

- **`BreadcrumbList`**, on every page. The trail comes from the page's
  rendered output path, never from `SUMMARY.md`, and each crumb's label
  is the real chapter name read out of that page's own rendered
  `<title>`. Deriving labels from the file name instead produced "Cray
  Hpc" and "Ti Compilers", which is worse than the trail Google already
  builds from the URL, and label control is the only thing this type
  buys. An ancestor directory with no `index.html` of its own is left
  out of the trail rather than pointed at the site root: a crumb that
  links somewhere its label does not describe is a false statement in
  machine-readable form.
- **`SoftwareApplication`**, on the home page only, identifying Bear as
  a piece of software with a name, description, URL, and a `sameAs`
  link to the GitHub repository.

No `aggregateRating`, `review`, or similar field is added to the
`SoftwareApplication` block: Bear collects no such data, and inventing
one to make the block "richer" would be a fabricated claim in machine
readable form, which is worse than emitting no rating at all.

## Consequences

State the expected gain honestly, because both remaining types are easy
to over-sell:

- **`BreadcrumbList`**: Google already derives a breadcrumb trail for
  search snippets from the URL path itself, with or without structured
  data. Adding an explicit `BreadcrumbList` buys label control over the
  words shown in that trail; it does not buy new eligibility for a rich
  result that would not otherwise appear.
- **`SoftwareApplication`**: without ratings or a price, this block is
  unlikely to produce any visible rich result at all. It is emitted for
  entity clarity, so that a crawler or assistant that does parse
  JSON-LD has an unambiguous, structured statement of what Bear is and
  where its canonical home and source are, not because it is expected
  to change how a result renders.
- Both are close to free specifically because the same post-processing
  pass already rewrites every page's `<head>` to add the canonical link
  (see `scripts/postprocess-docs-site.sh`). Adding two more `<script>`
  blocks to a pass that already opens and rewrites every file is a
  marginal cost. Neither is a promise of ranking lift, and this entry
  should not be read as one.
- LLM crawlers (the assistants that answer "how do I generate
  compile_commands.json for X") read rendered prose, not JSON-LD.
  Nothing here is expected to influence whether an AI assistant cites a
  page. That outcome depends on the answer-first writing rule in
  `site/CLAUDE.md` (the first paragraph of a page gives the working
  answer), not on structured data. Structured data is a Google-search
  lever, and on this site, given the two rejected rich-result types
  above, a small one.
- When Google ships a new rich-result type that plausibly fits a
  recipe or FAQ page, re-evaluate against the current search gallery
  documentation rather than against this entry's snapshot of it: the
  facts recorded above (dates, deprecation status) are what was true
  when this decision was made, not a permanent state of Google Search.

## References

- Post-processing implementation:
  `scripts/postprocess-docs-site.sh`
- https://developers.google.com/search/docs/appearance/structured-data/search-gallery
  - the current list of rich results Google supports
- `docs/rationale/docs-site-over-wiki.md` - the decision to have a site
  at all, including the "no lift is assumed" stance this entry extends
  to structured data specifically

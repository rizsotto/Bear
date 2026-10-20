# The site emits only BreadcrumbList and SoftwareApplication JSON-LD

## Context

The site build adds JSON-LD structured data to every rendered page,
in the same post-processing pass that adds the canonical link. Before
picking schema.org types, the obvious ones were checked against
Google's
[search gallery](https://developers.google.com/search/docs/appearance/structured-data/search-gallery),
because a valid schema.org type says nothing about whether Google still
rewards it.

Three types look like a fit for this site and were rejected. All three
are still valid, and Google parses them without penalty, but they no
longer produce any rich result:

- `HowTo`. The recipe pages are numbered-step task pages, a textbook
  `HowTo`. Google dropped `HowTo` rich results from mobile search in
  August 2023 and from desktop in September 2023.
- `FAQPage`. An FAQ page is a textbook `FAQPage`. Google limited these
  rich results to government and health sites in August 2023, then
  deprecated the feature on 2026-05-07. Search Console reporting for it
  ends in June 2026 and API support in August 2026.
- `WebSite` with a `SearchAction`, the sitelinks search box. Google
  retired it in November 2024. It is a common default addition, so it
  is named here before someone adds it out of habit.

Someone will look at the recipe pages, see the structural match, and
propose this markup again. That is why this entry exists.

For `BreadcrumbList` labels, deriving them from file names was tried and
rejected. It produced "Cray Hpc" and "Ti Compilers", worse than the
trail Google already builds from the URL. Label control is the only
thing the type buys. For a directory with no `index.html`, pointing its
crumb at the site root was rejected too: a crumb that links to a page its
label does not describe is a false statement in machine-readable form.

## Decision

The site emits `BreadcrumbList` on every page, with each label taken
from that page's own rendered `<title>`, and `SoftwareApplication` on
the home page only. A directory with no page of its own gets no crumb,
and the `SoftwareApplication` block has no rating or review fields,
because Bear collects no such data.

## Consequences

- Google already builds a breadcrumb trail from the URL. An explicit
  `BreadcrumbList` controls the words in that trail. It does not make
  the page eligible for any new rich result.
- Without a rating or a price, `SoftwareApplication` is unlikely to
  show any rich result. It is there so a crawler that does read JSON-LD
  gets a clear statement of what Bear is and where it lives.
- Both are cheap because the pass already rewrites every page's
  `<head>`. Neither is a promise of ranking lift.
- LLM assistants read rendered prose, not JSON-LD. Whether they cite a
  page depends on the answer-first writing rule in
  [site/CLAUDE.md](../../site/CLAUDE.md), not on structured data.
- The dates above are what was true when this was decided. Revisit
  against the current search gallery, not this entry, when Google ships
  a rich result that fits a recipe or FAQ page.

## References

- [docs-site-over-wiki](docs-site-over-wiki.md), the decision to have a site, and the "no lift is assumed" stance this extends
- [Google search gallery](https://developers.google.com/search/docs/appearance/structured-data/search-gallery), the rich results Google supports
- [scripts/postprocess-docs-site.sh](../../scripts/postprocess-docs-site.sh), the pass that emits the JSON-LD

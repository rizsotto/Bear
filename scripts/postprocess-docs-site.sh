#!/bin/sh
# Post-process the built documentation site under `site/book/` for search
# and AI-assistant discovery: generate a sitemap and robots.txt, give
# each page its own meta description, and inject a canonical link plus
# JSON-LD structured data into every rendered page's <head>.
#
# Run after `mdbook build site` (the same build that
# `./scripts/check-docs-site.sh` performs):
#     mdbook build site
#     ./scripts/postprocess-docs-site.sh
#
# Why this cannot be done through mdBook's theme/head.hbs:
#   `head.hbs` is rendered once per SOURCE page and only exposes the
#   source `.md` path (`{{ path }}`); mdBook does not expose the
#   computed output URL to templates (verified against mdBook 0.5.4), so
#   a per-page canonical href cannot be produced there. This script
#   instead edits the already-rendered HTML in `site/book/`, where the
#   final path is simply the file's own location on disk.
#
# What it does, in order:
#   1. Writes `site/book/sitemap.xml`, one <url> per rendered page, in
#      the site's live https://rizsotto.github.io/Bear/ namespace.
#      Excluded: any page mdBook itself marks
#      `<meta name="robots" content="noindex">` (this covers the
#      whole-book concatenation `print.html` and the sidebar-iframe
#      fragment `toc.html`), `404.html` (error pages do not belong in a
#      sitemap regardless of any meta tag), and any redirect stub (a
#      page whose only job is a meta-refresh to another URL). Listing a
#      noindex or error page is a Search Console "submitted URL not
#      indexed" report waiting to happen.
#   2. Writes `site/book/robots.txt`, allowing all crawlers and pointing
#      them at the sitemap.
#   3. Rewrites every rendered page's `<meta name="description">` (mdBook
#      stamps the single book.toml description onto all of them) with a
#      description built from that page's OWN prose, read out of `<main>`
#      in document order: `<p>` elements are collected (skipping anything
#      nested inside `<pre>`, `<table>`, `<ul>`, `<ol>`, or `<blockquote>`,
#      none of which is prose) and joined with a single space until the
#      joined text reaches roughly 120 characters or the page runs out of
#      paragraphs, because this site's answer-first authoring rule means
#      a lone first paragraph is often just a lead-in clause ending in
#      `:` ("Run this from the directory holding the Makefile:"), too
#      thin on its own to work as a search snippet. Tags are stripped and
#      entities decoded as the paragraphs are read, and the joined text
#      is truncated to about 155 characters on a word boundary. See
#      compute_description below for the exact rule, including the
#      trailing-colon guard, and its fallback to the book-level
#      description (used for the home page, and any page with no usable
#      paragraph at all, such as 404.html). A page with no `<meta
#      name="description">` element at all (the sidebar-iframe fragment
#      toc.html) is left untouched rather than gaining one, since
#      nothing else in the pipeline expects it to carry meta tags.
#   4. Injects, into every rendered page's <head>, a
#      `<link rel="canonical">` to that page's own live URL, plus
#      JSON-LD in a `<script type="application/ld+json">` block: a
#      `BreadcrumbList`, and, on the home page only, a
#      `SoftwareApplication` block. The pages that make up a page's
#      breadcrumb trail come from its OUTPUT path (SUMMARY.md is never
#      parsed), but each crumb's label is the real chapter name read out
#      of that page's own rendered <title> (see get_page_title below); a
#      title-cased path segment is only a fallback for the rare page
#      with no <title> to read, or no index page for a directory
#      segment. See docs/rationale/structured-data-scope.md for which
#      other structured-data types were considered and rejected, and
#      why. A single HTML comment marker guards the whole injected
#      block, so re-running this script against already-processed
#      output is a no-op (checked per page, not by diffing the whole
#      file).
#
# mdBook must already have built `site/book/`. This script does not
# invoke mdbook itself, so it can be re-run standalone against existing
# output.
#
# Exit codes:
#   0 - sitemap, robots.txt, and per-page rewrite all succeeded
#   2 - invocation error: `site/book` is missing, `awk` or `python3` is
#       not on PATH, or a rendered page has no `<head>` element to
#       inject into

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
book_dir="${repo_root}/site/book"

base_url="https://rizsotto.github.io/Bear/"
marker="<!-- postprocess-docs-site: generated, do not edit by hand -->"
# The single description book.toml stamps onto every page (see
# site/book.toml). Used verbatim for the home page and the
# SoftwareApplication JSON-LD, and as the fallback for any other page
# whose first paragraph cannot be turned into a description (see
# compute_description below).
book_description="Generate compile_commands.json for any C or C++ build."

if [ ! -d "${book_dir}" ]; then
    echo "error: no book directory found at ${book_dir}" >&2
    echo "hint: run 'mdbook build site' first" >&2
    exit 2
fi

if ! command -v awk >/dev/null 2>&1; then
    echo "error: awk not found on PATH" >&2
    exit 2
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "error: python3 not found on PATH" >&2
    exit 2
fi

page_list="$(mktemp)"
tmp_sitemap="$(mktemp)"
tmp_block="$(mktemp)"
tmp_page="$(mktemp)"
trap 'rm -f "${page_list}" "${tmp_sitemap}" "${tmp_block}" "${tmp_page}"' EXIT

find "${book_dir}" -name '*.html' | sort >"${page_list}"

# --- 1. sitemap.xml ---------------------------------------------------

{
    printf '<?xml version="1.0" encoding="UTF-8"?>\n'
    printf '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
} >"${tmp_sitemap}"

while read -r page; do
    relative="${page#"${book_dir}"/}"
    base="$(basename "${page}")"

    if [ "${base}" = "404.html" ]; then
        continue
    fi
    if grep -q '<meta name="robots" content="noindex">' "${page}"; then
        continue
    fi
    if grep -q 'http-equiv="refresh"' "${page}"; then
        continue
    fi

    if [ "${relative}" = "index.html" ]; then
        url="${base_url}"
    else
        url="${base_url}${relative}"
    fi
    printf '  <url><loc>%s</loc></url>\n' "${url}"
done <"${page_list}" >>"${tmp_sitemap}"

printf '</urlset>\n' >>"${tmp_sitemap}"
mv "${tmp_sitemap}" "${book_dir}/sitemap.xml"

# --- 2. robots.txt ------------------------------------------------------

{
    printf 'User-agent: *\n'
    printf 'Allow: /\n'
    printf 'Sitemap: %ssitemap.xml\n' "${base_url}"
} >"${book_dir}/robots.txt"

# --- 3. per-page <head> rewrite: description, canonical, JSON-LD -------

# Derives a page's own meta description from its own prose. This site's
# answer-first rule (site/CLAUDE.md content rule 3) means a page's FIRST
# paragraph is often just a lead-in clause to a code block ("Run this
# from the directory holding the Makefile:"), too thin on its own to
# read as a search snippet, so paragraphs are collected in document
# order and joined with a single space until the joined text reaches
# about 120 characters or the page runs out of paragraphs. Only <p>
# elements directly inside <main> count as prose: anything nested inside
# <pre> (a code block), <table>, <ul>/<ol> (list items, including a
# "loose" list whose items render as inner <p> tags), or <blockquote> (an
# admonition) is skipped, and the leading Diataxis-type comment and the
# <h1> page header are skipped too, since neither is a <p>. Tags are
# stripped and entities decoded per paragraph as they are read (Python's
# HTMLParser with convert_charrefs=True does both), and the final joined
# text is truncated to about 155 characters on a word boundary (an ASCII
# "..." marks a truncation; text already at or under the limit is left
# alone). A joined text that still ends with ":" means the loop ran out
# of paragraphs before completing a real sentence; rather than ship a
# dangling lead-in clause, it is cut back to the end of the last full
# sentence, or dropped entirely if there is no earlier sentence to cut
# back to. The final text is escaped for an HTML attribute (& and " at
# minimum). Prints nothing, rather than a bad description, whenever none
# of this yields usable text (no <main>, no <p> inside it, or a
# trailing-colon page with no earlier sentence); the caller then falls
# back to the book-level description. That fallback is also what covers
# 404.html (its first paragraph reads fine in isolation, but a page
# that is deliberately excluded from the sitemap as "not meant to be
# indexed" should not get bespoke SERP copy either).
#   $1 = absolute path to the page's rendered HTML file
compute_description() {
    page_file="$1"
    python3 - "${page_file}" <<'PYEOF'
import sys
from html.parser import HTMLParser


class ProseExtractor(HTMLParser):
    """Collects the text of every <p> directly inside <main>, in document
    order, skipping anything nested inside a code block, table, list, or
    admonition (see compute_description's shell comment for why)."""

    SKIP_TAGS = ("pre", "table", "ul", "ol", "blockquote")

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.main_depth = 0
        self.skip_depth = 0
        self.in_p = False
        self.buf = []
        self.paragraphs = []

    def handle_starttag(self, tag, attrs):
        if tag == "main":
            self.main_depth += 1
            return
        if self.main_depth == 0:
            return
        if tag in self.SKIP_TAGS:
            self.skip_depth += 1
            return
        if self.skip_depth == 0 and tag == "p":
            self.in_p = True
            self.buf = []

    def handle_endtag(self, tag):
        if tag == "main":
            self.main_depth = max(0, self.main_depth - 1)
            return
        if self.main_depth == 0:
            return
        if tag in self.SKIP_TAGS:
            self.skip_depth = max(0, self.skip_depth - 1)
            return
        if self.skip_depth == 0 and tag == "p" and self.in_p:
            text = " ".join("".join(self.buf).split())
            if text:
                self.paragraphs.append(text)
            self.in_p = False
            self.buf = []

    def handle_data(self, data):
        if self.in_p and self.skip_depth == 0:
            self.buf.append(data)


def cut_dangling_colon(joined):
    """joined ends with ":" (a lead-in with nothing after it in the
    collected text yet). Cuts back to the end of the last complete
    sentence inside it, or returns "" if there is no earlier sentence to
    cut back to."""
    cut_at = max(joined.rfind(". "), joined.rfind("! "), joined.rfind("? "))
    return joined[: cut_at + 1] if cut_at != -1 else ""


with open(sys.argv[1], encoding="utf-8") as f:
    source = f.read()

parser = ProseExtractor()
parser.feed(source)
paragraphs = parser.paragraphs
if not paragraphs:
    sys.exit(0)

# A paragraph both SHORT and colon-terminated is a bare lead-in to
# whatever code block follows it ("Then confirm it is on your PATH:"),
# not free-standing prose, so a LATER one (the page's own first
# paragraph is always kept regardless, see below) is dropped rather than
# appended: installation.html has three such paragraphs in a row before
# its first real sentence, and joining all three reads as three
# disconnected instructions, not "one or two whole sentences". A LONG
# paragraph that happens to end in ":" (introducing a JSON example, say)
# is still real content and is kept; if it is the last thing collected,
# the dangling-colon handling below trims it back to its last complete
# sentence instead.
lead_in_limit = 80
target = 120
eligible = paragraphs[:1] + [
    p
    for p in paragraphs[1:]
    if not (p.endswith(":") and len(p) < lead_in_limit)
]

text = ""
collected = []
for paragraph in eligible:
    collected.append(paragraph)
    joined = " ".join(collected)
    if len(joined) < target:
        continue
    if joined.endswith(":"):
        resolved = cut_dangling_colon(joined)
        if not resolved:
            # The target length was reached only by way of a dangling
            # lead-in with no earlier sentence anywhere in it yet (e.g.
            # a single long compound clause: "Source the environment
            # script, then run the build exactly as you would for any
            # other compiler:"). Keep pulling paragraphs instead of
            # settling for that.
            continue
        text = resolved
    else:
        text = joined
    break
else:
    # Ran out of paragraphs without a clean stop; use whatever was
    # collected, with the same dangling-colon cleanup as a last resort.
    joined = " ".join(collected)
    text = cut_dangling_colon(joined) if joined.endswith(":") else joined

if not text:
    sys.exit(0)

limit = 155
if len(text) > limit:
    cut = text[:limit]
    last_space = cut.rfind(" ")
    if last_space > 0:
        cut = cut[:last_space]
    text = cut.rstrip(" .,;:-") + "..."

# Escape for an HTML attribute. Order matters: escape "&" first, so the
# "&quot;" the next line introduces is not itself re-escaped.
escaped = text.replace("&", "&amp;").replace('"', "&quot;")
print(escaped)
PYEOF
}

# Capitalizes the first letter of each hyphen/underscore-separated word,
# e.g. "getting-started" -> "Getting Started". Fallback only, used when a
# path segment has no corresponding rendered page to read a real title
# from (see get_page_title below, which is what breadcrumb labels use
# whenever a page exists).
title_case() {
    printf '%s' "$1" | tr -- '-_' '  ' | awk '{
        out = ""
        for (i = 1; i <= NF; i++) {
            w = $i
            first = toupper(substr(w, 1, 1))
            rest = substr(w, 2)
            out = out (i > 1 ? " " : "") first rest
        }
        print out
    }'
}

# Reads a rendered page's own chapter name out of its <title> element,
# e.g. "<title>Recipes - Bear</title>" -> "Recipes". This is reading the
# BUILT OUTPUT, not SUMMARY.md: mdBook titles every page
# "<chapter name> - <book title>" (book title is "Bear", from
# book.toml), so stripping the " - Bear" suffix recovers the exact
# query-shaped chapter name a human sees in the browser tab, instead of
# a title-cased guess at the filename. <title> is used rather than <h1>
# because <h1> can carry inline markup (e.g. `<code>` around
# compile_commands.json); <title> is the same string as plain text.
# Prints nothing (caller falls back to title_case) if the page has no
# <title>, which is the case for a couple of internal mdBook fragments
# (the sidebar iframe toc.html has none).
get_page_title() {
    file="$1"

    raw="$(grep -o '<title>[^<]*</title>' "${file}" 2>/dev/null | head -n1)"
    [ -z "${raw}" ] && return 0
    raw="${raw#<title>}"
    raw="${raw%</title>}"
    raw="${raw% - Bear}"

    # Decode the handful of entities mdBook can put in a title (order
    # matters: &amp; must be decoded last, or a literal "&lt;" produced
    # by decoding &amp;lt; would be re-decoded into "<").
    raw="$(printf '%s' "${raw}" | sed \
        -e 's/&lt;/</g' \
        -e 's/&gt;/>/g' \
        -e 's/&quot;/"/g' \
        -e "s/&#39;/'/g" \
        -e 's/&amp;/\&/g')"

    # JSON-escape backslash and double quote so this can be dropped
    # straight into a JSON string literal. Backslash first, so the
    # backslash the quote rule introduces is not itself re-escaped.
    raw="$(printf '%s' "${raw}" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g')"

    printf '%s' "${raw}"
}

# Prints one ListItem JSON object (no surrounding brackets, no comma).
#   $1 = position  $2 = name  $3 = item URL
make_breadcrumb_item() {
    printf '{"@type":"ListItem","position":%d,"name":"%s","item":"%s"}' \
        "$1" "$2" "$3"
}

# Builds the itemListElement array (without the enclosing brackets) of a
# BreadcrumbList for one page. Every trail starts at Home, whose item is
# the truthful site root URL. Each further crumb's label is the real
# chapter name read from that page's own <title> (get_page_title); the
# output path is used only to find WHICH pages belong in the trail. A
# directory segment with no index.html contributes NO crumb at all: a
# crumb must link to a real page, and there is no honest URL to give a
# "Platforms"-style crumb when platforms/index.html does not exist, so
# it is dropped rather than pointed at the site root or a fabricated
# page. Positions are renumbered contiguously from 1 as crumbs are
# added, so a drop never leaves a gap.
#   $1 = page path without the .html extension, e.g. "recipes/docker",
#        "recipes/index", "platforms/linux", "index", "faq"
#   $2 = that page's own canonical URL (used as the final crumb's item)
#   $3 = absolute path to that page's own rendered HTML file
build_breadcrumb_items() {
    path_no_ext="$1"
    page_canonical="$2"
    page_file="$3"

    old_ifs="${IFS}"
    IFS='/'
    set -- ${path_no_ext}
    IFS="${old_ifs}"

    count=$#
    eval "last_seg=\${${count}}"
    # A section's own index page (e.g. "recipes/index") is represented by
    # its directory segment, not by a trailing literal "index" crumb.
    if [ "${last_seg}" = "index" ] && [ "${count}" -gt 1 ]; then
        count=$((count - 1))
    fi

    # The home page itself: a single fixed "Home" crumb, nothing more.
    if [ "${count}" -eq 1 ]; then
        eval "seg=\${1}"
        if [ "${seg}" = "index" ]; then
            make_breadcrumb_item 1 "Home" "${page_canonical}"
            return 0
        fi
    fi

    pos=1
    items="$(make_breadcrumb_item "${pos}" "Home" "${base_url}")"

    prefix=""
    i=1
    while [ "${i}" -lt "${count}" ]; do
        eval "seg=\${${i}}"
        prefix="${prefix}${seg}/"
        candidate="${book_dir}/${prefix}index.html"
        if [ -f "${candidate}" ]; then
            name="$(get_page_title "${candidate}")"
            [ -z "${name}" ] && name="$(title_case "${seg}")"
            pos=$((pos + 1))
            items="${items},$(make_breadcrumb_item "${pos}" "${name}" "${base_url}${prefix}index.html")"
        fi
        # else: no index page for this directory segment, drop the
        # crumb entirely rather than link a page that does not exist.
        i=$((i + 1))
    done

    eval "seg=\${${count}}"
    name="$(get_page_title "${page_file}")"
    [ -z "${name}" ] && name="$(title_case "${seg}")"
    pos=$((pos + 1))
    items="${items},$(make_breadcrumb_item "${pos}" "${name}" "${page_canonical}")"

    printf '%s' "${items}"
}

while read -r page; do
    if grep -qF "${marker}" "${page}"; then
        # Already processed by a previous run of this script: skip, so
        # re-running never double-injects.
        continue
    fi

    if ! grep -q '<head' "${page}"; then
        echo "error: no <head> element in ${page#"${repo_root}"/}" >&2
        exit 2
    fi

    relative="${page#"${book_dir}"/}"
    path_no_ext="${relative%.html}"

    if [ "${relative}" = "index.html" ]; then
        canonical="${base_url}"
        is_home=1
    else
        canonical="${base_url}${relative}"
        is_home=0
    fi

    # The home page keeps the book-level description; it is already the
    # right sentence for the site root. 404.html gets it too: part 1
    # above excludes 404.html from the sitemap on the grounds that an
    # error page is not meant to be indexed regardless of its content,
    # and the same reasoning says it should not get bespoke SERP copy
    # either. Every other page tries its own first paragraph first.
    if [ "${is_home}" -eq 1 ] || [ "${relative}" = "404.html" ]; then
        description="${book_description}"
    else
        description="$(compute_description "${page}")"
        [ -z "${description}" ] && description="${book_description}"
    fi

    # A page with no <meta name="description"> element at all (the
    # sidebar-iframe fragment toc.html) is left untouched: there is
    # nothing to replace, and nothing downstream expects it to have one.
    if grep -q '<meta name="description" content="' "${page}"; then
        awk -v desc="${description}" '
            /<meta name="description" content="/ {
                print "        <meta name=\"description\" content=\"" desc "\">"
                next
            }
            { print }
        ' "${page}" >"${tmp_page}"
        mv "${tmp_page}" "${page}"
    fi

    breadcrumb_items="$(build_breadcrumb_items "${path_no_ext}" "${canonical}" "${page}")"
    breadcrumb_json=$(printf '{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[%s]}' \
        "${breadcrumb_items}")

    {
        printf '%s\n' "${marker}"
        printf '<link rel="canonical" href="%s">\n' "${canonical}"
        printf '<script type="application/ld+json">\n'
        printf '%s\n' "${breadcrumb_json}"
        printf '</script>\n'
        if [ "${is_home}" -eq 1 ]; then
            software_json=$(printf '{"@context":"https://schema.org","@type":"SoftwareApplication","name":"Bear","description":"%s","url":"https://rizsotto.github.io/Bear/","applicationCategory":"DeveloperApplication","operatingSystem":"Linux, macOS, FreeBSD, OpenBSD, NetBSD, DragonFly BSD, Windows","sameAs":"https://github.com/rizsotto/Bear"}' \
                "${book_description}")
            printf '<script type="application/ld+json">\n'
            printf '%s\n' "${software_json}"
            printf '</script>\n'
        fi
    } >"${tmp_block}"
    block="$(cat "${tmp_block}")"

    awk -v inject="${block}" '
        !done && index($0, "</head>") > 0 {
            print inject
            done = 1
        }
        { print }
    ' "${page}" >"${tmp_page}"
    mv "${tmp_page}" "${page}"
done <"${page_list}"

echo "OK: sitemap.xml, robots.txt written; canonical and JSON-LD injected"

#!/bin/sh
# Post-process the built documentation site under `site/book/` for search
# and AI-assistant discovery: generate a sitemap and robots.txt, and
# inject a canonical link plus JSON-LD structured data into every
# rendered page's <head>.
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
#   3. Injects, into every rendered page's <head>, a
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
#   0 - sitemap, robots.txt, and per-page injection all succeeded
#   2 - invocation error: `site/book` is missing, or a rendered page has
#       no `<head>` element to inject into

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
book_dir="${repo_root}/site/book"

base_url="https://rizsotto.github.io/Bear/"
marker="<!-- postprocess-docs-site: generated, do not edit by hand -->"

if [ ! -d "${book_dir}" ]; then
    echo "error: no book directory found at ${book_dir}" >&2
    echo "hint: run 'mdbook build site' first" >&2
    exit 2
fi

if ! command -v awk >/dev/null 2>&1; then
    echo "error: awk not found on PATH" >&2
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

# --- 3. per-page <head> injection ---------------------------------------

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
            software_json='{"@context":"https://schema.org","@type":"SoftwareApplication","name":"Bear","description":"Generate compile_commands.json for any C or C++ build.","url":"https://rizsotto.github.io/Bear/","applicationCategory":"DeveloperApplication","operatingSystem":"Linux, macOS, FreeBSD, OpenBSD, NetBSD, DragonFly BSD, Windows","sameAs":"https://github.com/rizsotto/Bear"}'
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

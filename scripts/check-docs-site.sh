#!/bin/sh
# Build the documentation site under `site/` and verify its internal
# links. This is the build-and-link check that `.github/workflows/pages.yml`
# runs; run it before committing any change under `site/`.
#
# Run from anywhere:
#     ./scripts/check-docs-site.sh
#
# The check has four parts:
#   1. `mdbook build` must succeed AND print no WARN or ERROR line.
#      Warnings are fatal on purpose: mdBook reports preprocessor and
#      include failures that way, and they would otherwise ship silently.
#      A chapter listed in SUMMARY.md whose file is missing is a hard
#      error because `book.toml` sets `build.create-missing = false`.
#      Only mdBook's log-level field is matched, not the words "warn" or
#      "error" anywhere in the output: the log echoes the absolute output
#      path, and a checkout under, say, /home/warner would otherwise
#      always fail.
#   2. Every inline Markdown link to a `.md` target must resolve to an
#      existing file. mdBook does NOT check this itself (verified against
#      mdBook 0.5.4: a link to a nonexistent page builds clean), and the
#      site uses no plugins, `mdbook-linkcheck` included. Scope: the scan
#      is textual, so it also reads fenced code blocks and HTML comments.
#      An example link to a deliberately nonexistent file inside a code
#      block would be reported as broken; there is no such instance
#      today. Reference-style link definitions are not scanned: by
#      convention they carry external URLs only.
#   3. Every page under `site/src/` must be listed in SUMMARY.md. mdBook
#      ignores files that are not, so such a page would never be served.
#      A commented-out SUMMARY entry does not count as listed, and a
#      `./page.md` entry counts the same as `page.md`. 404.md is exempt:
#      mdBook renders it as the not-found page, and listing it would put
#      it in the navigation.
#   4. Every absolute `/Bear/<page>.html` link in 404.md must have a
#      matching `<page>.md`. That page links absolutely because Pages
#      serves it for any missing path, so part 2 cannot follow its links.
#
# mdBook must be on PATH. Install the version pinned in
# `.github/workflows/pages.yml`, which is the single source of truth for
# it.
#
# Exit codes:
#   0 - the site builds clean and every link and page checks out
#   1 - at least one build warning, broken link, or unlisted page
#   2 - invocation error (mdbook missing, site directory not found)

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
site_dir="${repo_root}/site"
src_dir="${site_dir}/src"
summary="${src_dir}/SUMMARY.md"

if [ ! -f "${site_dir}/book.toml" ]; then
    echo "error: no book.toml found at ${site_dir}" >&2
    exit 2
fi

if ! command -v mdbook >/dev/null 2>&1; then
    echo "error: mdbook not found on PATH" >&2
    echo "hint: install the version pinned in .github/workflows/pages.yml" >&2
    exit 2
fi

log="$(mktemp)"
problems="$(mktemp)"
toc="$(mktemp)"
trap 'rm -f "${log}" "${problems}" "${toc}"' EXIT

# 1. Build, then treat any WARN or ERROR log line as a failure. mdBook
#    0.5 prints the level as the first field of the line.
if ! mdbook build "${site_dir}" >"${log}" 2>&1; then
    cat "${log}"
    echo "FAILED: mdbook build returned a non-zero status" >&2
    exit 1
fi
cat "${log}"
if grep -E '^[[:space:]]*(WARN|ERROR)[[:space:]]' "${log}" >/dev/null 2>&1; then
    echo "FAILED: mdbook printed WARN or ERROR lines (above)" >&2
    exit 1
fi

# 2. Inline links to Markdown files must resolve. An optional link title
#    (`[text](page.md "title")`) and an optional `#anchor` are stripped
#    before the file is looked up.
find "${src_dir}" -name '*.md' | while read -r page; do
    page_dir="$(dirname "${page}")"
    grep -oE '\]\([^)]*\.md[^)]*\)' "${page}" 2>/dev/null |
        sed -E 's/^\]\(//; s/\)$//; s/[[:space:]].*$//; s/#.*$//' |
        grep -vE '^[a-zA-Z][a-zA-Z0-9+.-]*:' |
        while read -r target; do
            if [ ! -e "${page_dir}/${target}" ]; then
                echo "BROKEN LINK: ${page#"${repo_root}"/} -> ${target}" \
                    >>"${problems}"
            fi
        done
done

# 3. Every page must appear in the table of contents. Collect the paths
#    SUMMARY.md actually links from its chapter lines: HTML comments are
#    removed first (a commented-out entry is not served), single-line
#    ones before the multi-line range so that a self-contained comment
#    does not open a range that swallows the rest of the file. Only lines
#    that are a list item or a prefix/suffix chapter are read, and a
#    leading `./` is normalized away.
sed -e 's/<!--.*-->//g' -e '/<!--/,/-->/d' "${summary}" |
    grep -E '^[[:space:]]*([-*+][[:space:]]+)?\[' |
    grep -oE '\]\([^)]*\)' |
    sed -E 's/^\]\(//; s/\)$//; s/[[:space:]].*$//; s/#.*$//; s|^\./||' \
        >"${toc}"

find "${src_dir}" -name '*.md' | while read -r page; do
    relative="${page#"${src_dir}"/}"
    # SUMMARY.md is the table of contents itself. 404.md is rendered by
    # mdBook as the not-found page and must NOT be listed, or it would
    # appear in the navigation as a chapter.
    if [ "${relative}" = "SUMMARY.md" ] || [ "${relative}" = "404.md" ]; then
        continue
    fi
    if ! grep -qxF "${relative}" "${toc}"; then
        echo "NOT IN SUMMARY.md: ${relative}" >>"${problems}"
    fi
done

# 4. The not-found page links with absolute `/Bear/...` URLs, because it
#    is served for any missing path and relative links would resolve
#    against that path. Part 2 cannot check those, so check them here:
#    every `/Bear/<path>.html` must have a `<path>.md` under src/.
if [ -f "${src_dir}/404.md" ]; then
    grep -oE '\(/Bear/[^)]*\.html\)' "${src_dir}/404.md" 2>/dev/null |
        sed -E 's|^\(/Bear/||; s|\.html\)$||' |
        while read -r target; do
            if [ ! -e "${src_dir}/${target}.md" ]; then
                echo "BROKEN 404 LINK: /Bear/${target}.html has no ${target}.md" \
                    >>"${problems}"
            fi
        done
fi

if [ -s "${problems}" ]; then
    cat "${problems}" >&2
    echo "FAILED: the site has broken links or unlisted pages" >&2
    exit 1
fi

echo "OK: site builds clean, links resolve, every page is in SUMMARY.md"

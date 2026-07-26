#!/bin/sh
# Build the documentation site under `site/` and verify its internal
# links. This is the build-and-link check that `.github/workflows/pages.yml`
# runs; run it before committing any change under `site/`.
#
# Run from anywhere:
#     ./scripts/check-docs-site.sh
#
# The check has six parts:
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
#   5. `src/supported-compilers.md` is generated, in part, from
#      `crates/bear/compilers/*.yaml` by
#      `scripts/generate-supported-compilers.py`. Regenerate it into a
#      scratch file and diff against the committed page: any difference
#      means the YAML changed, or someone hand-edited the generated
#      block, without re-running the generator. Uses only the local
#      checkout (no network access) and never writes to the committed
#      page.
#   6. `site/src/configuration.md` must not name a configuration key
#      that `man/bear.1.md` does not also mention. `man/bear.1.md` is
#      the owner of configuration keys and defaults (see
#      `site/CLAUDE.md`), so configuration.md is reference content that
#      overlaps it by design; this is the drift check for that overlap,
#      in place of a generator because there is no schema export to
#      generate the page from. Extraction rule: every backticked,
#      dot-separated, lowercase/underscore identifier of two or more
#      segments in configuration.md (for example `format.paths.directory`)
#      is a candidate key. For each one, its last segment (`directory`)
#      must appear as a whole word somewhere in man/bear.1.md. This
#      covers every key documented so far: man/bear.1.md always names a
#      key's last segment, even where it does not spell out the full
#      dotted path (compilers, sources, duplicates, and headers keys are
#      written there under their own section heading, undotted). It does
#      NOT cover a single-segment top-level key (`schema` is not
#      dotted, so it is not extracted), and it does not verify a key's
#      accepted values or default, only that the key name itself is not
#      invented. A whole-word match is deliberately loose: it does not
#      confirm the man page documents the key as configuration (a
#      coincidental prose word would also pass), so this check only
#      catches a key that man/bear.1.md never mentions at all, such as a
#      typo or an invented key. That one-directional, name-only check is
#      enough to catch the drift that matters here: a key added to the
#      site page without ever being added to the man page.
#
# mdBook must be on PATH. Install the version pinned in
# `.github/workflows/pages.yml`, which is the single source of truth for
# it.
#
# Exit codes:
#   0 - the site builds clean, every link and page checks out, the
#       generated compiler page is up to date, and every configuration
#       key on configuration.md is also named in the man page
#   1 - at least one build warning, broken link, unlisted page, a
#       stale generated page, or a configuration key missing from the
#       man page
#   2 - invocation error (mdbook or python3 missing, site directory not
#       found)

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

if ! command -v python3 >/dev/null 2>&1; then
    echo "error: python3 not found on PATH" >&2
    exit 2
fi

log="$(mktemp)"
problems="$(mktemp)"
toc="$(mktemp)"
scratch="$(mktemp)"
keys="$(mktemp)"
trap 'rm -f "${log}" "${problems}" "${toc}" "${scratch}" "${keys}"' EXIT

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
                continue
            fi
            # A link that resolves OUTSIDE src/ exists on disk but not in
            # the built book: mdBook rewrites it to a sibling .html that
            # was never rendered, and on the deployed site it escapes the
            # book root entirely. Repo files must be linked by URL.
            resolved="$(cd "${page_dir}" && cd "$(dirname "${target}")" \
                && pwd)/$(basename "${target}")"
            case "${resolved}" in
                "${src_dir}"/*) ;;
                *)
                    echo "LINK ESCAPES src/: ${page#"${repo_root}"/} -> ${target}" \
                        >>"${problems}"
                    ;;
            esac
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

# 5. supported-compilers.md must match what the generator produces from
#    the current compiler YAML right now. Render into a scratch file
#    (the generator's own template read comes from the committed page,
#    so this never modifies it) and diff.
if ! python3 "${script_dir}/generate-supported-compilers.py" "${scratch}" \
        >"${log}" 2>&1; then
    cat "${log}" >&2
    echo "FAILED: generate-supported-compilers.py could not run" >&2
    exit 1
fi
if ! diff -u "${src_dir}/supported-compilers.md" "${scratch}" >"${log}"; then
    cat "${log}" >&2
    echo "FAILED: supported-compilers.md is stale" >&2
    echo "fix: python3 scripts/generate-supported-compilers.py" >&2
    exit 1
fi

# 6. configuration.md must not name a configuration key that
#    man/bear.1.md never mentions. man/bear.1.md owns configuration keys
#    and defaults (see site/CLAUDE.md), so configuration.md necessarily
#    overlaps it; this is the drift check for that overlap, standing in
#    for a generator since there is no schema export to generate the
#    page from. Extraction: every backticked, dot-separated,
#    lowercase/underscore identifier of two or more segments in
#    configuration.md is a candidate key, for example
#    `format.paths.directory`. Only the LAST segment (`directory`) is
#    checked, as a whole word, against man/bear.1.md: that page always
#    names a key's last segment, even for sections where it does not
#    spell out the full dotted path (compilers, sources, duplicates, and
#    headers keys are written there under their own section heading,
#    undotted). See the header comment for what this rule does and does
#    not cover.
config_page="${src_dir}/configuration.md"
man_page="${repo_root}/man/bear.1.md"
if [ -f "${config_page}" ]; then
    if [ ! -f "${man_page}" ]; then
        echo "error: man page not found at ${man_page}" >&2
        exit 2
    fi
    grep -oE '`[a-z][a-z0-9_]*(\.[a-z0-9_]+)+`' "${config_page}" |
        sed -E 's/^`//; s/`$//' |
        sort -u >"${keys}"
    while read -r key; do
        leaf="${key##*.}"
        if ! grep -qw -F -- "${leaf}" "${man_page}"; then
            echo "CONFIG KEY NOT IN MAN PAGE: configuration.md names" \
                "'${key}' (checked as '${leaf}'), which man/bear.1.md" \
                "never mentions" >>"${problems}"
        fi
    done <"${keys}"
fi

if [ -s "${problems}" ]; then
    cat "${problems}" >&2
    echo "FAILED: configuration.md names a key man/bear.1.md does not mention" >&2
    exit 1
fi

echo "OK: site builds clean, links resolve, every page is in SUMMARY.md, supported-compilers.md is up to date, and every configuration.md key is in the man page"

#!/bin/sh
# Verify that every cross-reference in a requirement file resolves to an
# existing requirement or rationale entry, and that every link in a rationale
# entry to another entry or to a requirement file resolves, then print review
# hints for text that may not belong in a contract.
#
# A cross-reference is a backticked ID with a known area prefix (`output-append`),
# a relative Markdown link to a .md file in the same directory (with or
# without a ./ prefix), or a relative Markdown link to ../rationale/*.md.
# Area prefixes are taken from the existing filenames.
#
# In a rationale entry every reference is a Markdown link. A backticked ID
# that names an existing requirement (`output-append`) outside a link label
# is an unlinked reference and fails the check, printed as file:line.
#
# Review hints do not affect the exit code. They are heuristic: an ID written
# with a .md suffix, and possible implementation leakage (Rust paths, source
# file names, crate names).
#
# Run from the repo root:
#     ./scripts/check-requirements-references.sh
#
# Exit codes:
#   0 - every cross-reference resolves, and every rationale reference is linked
#   1 - at least one cross-reference points to a missing file, or a
#       rationale entry names a requirement without linking it
#   2 - invocation error (e.g. script run from wrong directory)

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
req_dir="${repo_root}/docs/requirements"
rat_dir="${repo_root}/docs/rationale"

if [ ! -d "${req_dir}" ]; then
    echo "error: requirements directory not found at ${req_dir}" >&2
    exit 2
fi

files=""
for file in "${req_dir}"/*.md; do
    [ -e "${file}" ] || continue
    base="$(basename "${file}" .md)"

    # Skip the non-requirement files
    if [ "${base}" = "CLAUDE" ] || [ "${base}" = "TEMPLATE" ]; then
        continue
    fi

    files="${files} ${file}"
done

if [ -z "${files}" ]; then
    echo "error: no requirement files found in ${req_dir}" >&2
    exit 2
fi

# shellcheck disable=SC2086
prefixes="$(for file in ${files}; do basename "${file}" | sed 's/-.*//'; done | sort -u | paste -sd '|' -)"

dangling=0

# shellcheck disable=SC2086
ids="$(
    {
        grep -hoE "\`(${prefixes})-[a-z0-9-]+(\.md)?\`" ${files} | tr -d '`'
        grep -hoE '\]\((\./)?[a-z0-9-]+\.md(#[^)]*)?\)' ${files} | sed 's/^](//; s/^\.\///; s/[#)].*//'
    } | sed 's/\.md$//' | sort -u
)"

for id in ${ids}; do
    if [ ! -f "${req_dir}/${id}.md" ]; then
        echo "DANGLING: ${id}"
        grep -nE "\`${id}(\.md)?\`|\]\((\./)?${id}\.md" ${files} | sed "s|^${req_dir}/|    |"
        dangling=$((dangling + 1))
    fi
done

# shellcheck disable=SC2086
rationale="$(grep -hoE '\]\(\.\./rationale/[a-z0-9-]+\.md(#[^)]*)?\)' ${files} | sed 's|^](\.\./rationale/||; s/[#)].*//; s/\.md$//' | sort -u)"

for name in ${rationale}; do
    if [ ! -f "${rat_dir}/${name}.md" ]; then
        echo "DANGLING: ../rationale/${name}.md"
        grep -nE "\]\(\.\./rationale/${name}\.md" ${files} | sed "s|^${req_dir}/|    |"
        dangling=$((dangling + 1))
    fi
done

rat_files=""
for file in "${rat_dir}"/*.md; do
    [ -e "${file}" ] || continue
    base="$(basename "${file}" .md)"

    # Skip the non-entry files
    if [ "${base}" = "CLAUDE" ] || [ "${base}" = "TEMPLATE" ]; then
        continue
    fi

    rat_files="${rat_files} ${file}"
done

rat_links=""
if [ -n "${rat_files}" ]; then
    # shellcheck disable=SC2086
    rat_links="$(grep -hoE '\]\((\./|\.\./requirements/)?[a-z0-9-]+\.md(#[^)]*)?\)' ${rat_files} | sed 's/^](//; s/^\.\///; s/[#)].*//' | sort -u || true)"
fi

for link in ${rat_links}; do
    if [ ! -f "${rat_dir}/${link}" ]; then
        echo "DANGLING (from rationale): ${link}"
        # shellcheck disable=SC2086
        grep -nE "\]\((\./)?$(printf '%s' "${link}" | sed 's/\./\\./g')" ${rat_files} | sed "s|^${rat_dir}/|    |"
        dangling=$((dangling + 1))
    fi
done

unlinked=0
for file in ${rat_files}; do
    # Drop every Markdown link on the line, then look for backticked IDs.
    found="$(sed -E 's/\[[^]]*\]\([^)]*\)//g' "${file}" | grep -noE "\`(${prefixes})-[a-z0-9-]+\`" || true)"
    for hit in ${found}; do
        line="${hit%%:*}"
        id="$(printf '%s' "${hit#*:}" | tr -d '`')"
        if [ -f "${req_dir}/${id}.md" ]; then
            echo "UNLINKED (from rationale): $(basename "${file}"):${line}: ${id}"
            unlinked=$((unlinked + 1))
        fi
    done
done

# shellcheck disable=SC2086
grep -nE "\`(${prefixes})-[a-z0-9-]+\.md\`" ${files} | sed "s|^${req_dir}/|REVIEW (write the ID without .md): |" || true

# shellcheck disable=SC2086
grep -nE '::|\.(rs|yaml|yml)\b|\bcrate\b' ${files} | sed "s|^${req_dir}/|REVIEW (implementation detail?): |" || true

checked="$(printf '%s\n' "${ids}" "${rationale}" "${rat_links}" | grep -c . || true)"

echo
echo "Checked ${checked} cross-reference(s); ${dangling} dangling, ${unlinked} unlinked."

if [ "${dangling}" -gt 0 ] || [ "${unlinked}" -gt 0 ]; then
    exit 1
fi

#!/bin/sh
# Verify that every `implemented` requirement has at least one test
# referencing it via a `Requirements: <id>` tag.
#
# Run from the repo root:
#     ./scripts/check-requirements-coverage.sh
#
# Exit codes:
#   0 - every implemented requirement has at least one test reference
#   1 - at least one implemented requirement has zero references
#   2 - invocation error (e.g. script run from wrong directory)

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"

if [ ! -d "${repo_root}/docs/requirements" ]; then
    echo "error: requirements directory not found at ${repo_root}/docs/requirements" >&2
    exit 2
fi

# Space-separated search roots. Word splitting on the expansion below is
# intentional, so the paths must not contain spaces (they don't).
search_roots="${repo_root}/crates ${repo_root}/tests"

missing=0
checked=0

for file in "${repo_root}/docs/requirements"/*.md; do
    [ -e "${file}" ] || continue
    base="$(basename "${file}" .md)"

    # Skip the non-requirement files
    if [ "${base}" = "CLAUDE" ] || [ "${base}" = "TEMPLATE" ]; then
        continue
    fi

    # Extract status from YAML frontmatter (first match wins)
    status="$(awk '/^status:[[:space:]]*/ { sub(/^status:[[:space:]]*/, ""); print; exit }' "${file}")"

    if [ "${status}" != "implemented" ]; then
        continue
    fi

    checked=$((checked + 1))

    # Count matches across the search roots. A match is any line that contains
    # "Requirements:" followed (anywhere on the line) by the requirement ID.
    # Word-boundary on both sides prevents "output-path" from matching
    # "output-path-format".
    pattern="Requirements:.*\\b${base}\\b"

    # shellcheck disable=SC2086
    if grep -RnE "${pattern}" ${search_roots} >/dev/null 2>&1; then
        :
    else
        echo "MISSING: ${base} (status: implemented) has no test tagged with its ID"
        missing=$((missing + 1))
    fi
done

echo
echo "Checked ${checked} implemented requirement(s); ${missing} without coverage."

if [ "${missing}" -gt 0 ]; then
    exit 1
fi

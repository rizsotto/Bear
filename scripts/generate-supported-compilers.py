#!/usr/bin/env python3
# Generate the enumerated tables on site/src/reference/supported-compilers.md
# crates/bear/compilers/*.yaml, the actual source of truth for compiler
# recognition. Those YAML files already drive `bear semantic
# --print-compilers` and the codegen recognition table; this script keeps
# the site page from hand-copying (and silently drifting from) the same
# data.
#
# Run from anywhere:
#     python3 scripts/generate-supported-compilers.py
#
# Only the region between the BEGIN/END markers in
# site/src/reference/supported-compilers.md is replaced; everything outside it
# page's hand-written explanatory prose) is read from the current file
# and carried through unchanged. An optional argument overrides the
# output path (used by scripts/check-docs-site.sh to render into a
# scratch file without touching the committed page):
#
#     python3 scripts/generate-supported-compilers.py /tmp/scratch.md
#
# Parsing approach: crates/bear/compilers/*.yaml follows one fixed,
# narrow shape (see crates/bear/compilers/README.md) -- a handful of
# top-level keys, of which only four matter here (`type`, `compiler`,
# `recognize`, `ignore_when`); the rest (`flags`, `environment`,
# `options`, ...) are irrelevant to this page and skipped whole. No YAML
# library is available in this environment (`import yaml` fails; adding
# a dependency for four fields read from 23 well-known files is not
# justified), so this parses those four shapes directly with a small
# line-oriented state machine plus targeted regexes, rather than
# implementing anything close to general YAML.
#
# Exit codes:
#   0 - the page was (re)generated
#   1 - a YAML file could not be parsed in the expected shape, or the
#       markers are missing from the template page
#   2 - invocation error (compilers directory or template page missing)

import re
import sys
from pathlib import Path
from urllib.parse import urlparse

BEGIN_MARKER = (
    "<!-- BEGIN GENERATED: scripts/generate-supported-compilers.py -- "
    "DO NOT EDIT. Edits inside this block are lost the next time the "
    "script runs. -->"
)
END_MARKER = "<!-- END GENERATED -->"

TOP_LEVEL_KEY = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$")
QUOTED = re.compile(r'"([^"]*)"')


def parse_bracket_list(text: str) -> list[str]:
    """Extract the quoted strings out of a flow list like '["a", "b"]'."""
    start = text.index("[")
    end = text.index("]", start)
    return QUOTED.findall(text[start : end + 1])


def parse_compiler_yaml(path: Path) -> dict:
    """Parse the four shapes this page needs out of one compiler/wrapper
    YAML file: `type`, `compiler.id`, `recognize` entries, and
    `ignore_when`. Everything else in the file (flags, environment,
    slash_prefix, options, ...) is inert as far as this function is
    concerned: it is skipped by virtue of not being one of the four
    top-level keys tracked below.
    """
    kind = None
    compiler_id = None
    recognize: list[dict] = []
    ignore_when = {"executables": [], "flags": []}

    section = None
    current_entry = None
    in_references = False

    for raw_line in path.read_text().splitlines():
        if raw_line and not raw_line[0].isspace():
            m = TOP_LEVEL_KEY.match(raw_line)
            if m:
                section = m.group(1)
                if section == "type":
                    kind = m.group(2).strip()
                continue
            # A column-0 comment line outside any key: not a section
            # change, falls through to the indented handling below.

        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        if section == "compiler":
            if stripped.startswith("id:"):
                compiler_id = stripped.split(":", 1)[1].strip()
        elif section == "recognize":
            if stripped.startswith("- description:"):
                m = QUOTED.search(stripped)
                current_entry = {
                    "description": m.group(1) if m else "",
                    "references": [],
                    "executables": [],
                    "versioned": False,
                    "cross_compilation": False,
                }
                recognize.append(current_entry)
                in_references = False
            elif stripped == "references:":
                in_references = True
            elif in_references and stripped.startswith("-"):
                m = QUOTED.search(stripped)
                if m:
                    current_entry["references"].append(m.group(1))
            elif stripped.startswith("executables:"):
                in_references = False
                current_entry["executables"] = parse_bracket_list(stripped)
            elif stripped.startswith("versioned:"):
                in_references = False
                current_entry["versioned"] = "true" in stripped.split(":", 1)[1]
            elif stripped.startswith("cross_compilation:"):
                in_references = False
                current_entry["cross_compilation"] = (
                    "true" in stripped.split(":", 1)[1]
                )
        elif section == "ignore_when":
            if stripped.startswith("executables:"):
                ignore_when["executables"] = parse_bracket_list(stripped)
            elif stripped.startswith("flags:"):
                ignore_when["flags"] = parse_bracket_list(stripped)

    if kind not in ("compiler", "wrapper"):
        raise ValueError(f"{path}: no recognized 'type:' (got {kind!r})")
    if not recognize:
        raise ValueError(f"{path}: no 'recognize:' entries found")
    for entry in recognize:
        if not entry["executables"]:
            raise ValueError(f"{path}: recognize entry with no executables")
        if not entry["references"]:
            raise ValueError(f"{path}: recognize entry with no references")

    # Wrapper files carry no `compiler.id`: the launcher's own basename
    # (the file stem, by the directory's own convention) doubles as its
    # `as:` spelling -- see crates/bear/compilers/README.md, "the launcher
    # basename is emitted into WRAPPER_AS_NAMES so `as: mywrapper` is
    # accepted".
    family_id = compiler_id if kind == "compiler" else path.stem

    return {
        "id": family_id,
        "kind": kind,
        "recognize": recognize,
        "ignore_when": ignore_when,
    }


def load_families(compilers_dir: Path) -> tuple[list[dict], list[dict]]:
    families = [parse_compiler_yaml(p) for p in sorted(compilers_dir.glob("*.yaml"))]
    compilers = sorted(
        (f for f in families if f["kind"] == "compiler"), key=lambda f: f["id"]
    )
    wrappers = sorted(
        (f for f in families if f["kind"] == "wrapper"), key=lambda f: f["id"]
    )
    return compilers, wrappers


def doc_link(url: str) -> str:
    netloc = urlparse(url).netloc
    if netloc.startswith("www."):
        netloc = netloc[4:]
    return f"[{netloc}]({url})"


def doc_links(urls: list[str]) -> str:
    return ", ".join(doc_link(u) for u in urls)


def version_example(base: str) -> str:
    return f"`{base}-12`"


def cross_example(base: str) -> str:
    return f"`arm-linux-gnueabihf-{base}`"


def render_compiler_family(fam: dict) -> list[str]:
    lines = [f"#### `{fam['id']}`", ""]
    lines.append(f"Configuration `as:` value: `{fam['id']}`.")
    lines.append("")
    lines.append(
        "| Executable names | Recognized as | Version suffix | "
        "Cross-compilation prefix | Documentation |"
    )
    lines.append("|---|---|---|---|---|")
    for entry in fam["recognize"]:
        names = ", ".join(f"`{e}`" for e in entry["executables"])
        base = entry["executables"][0]
        version = version_example(base) if entry["versioned"] else "not recognized"
        cross = (
            cross_example(base) if entry["cross_compilation"] else "not recognized"
        )
        lines.append(
            f"| {names} | {entry['description']} | {version} | {cross} | "
            f"{doc_links(entry['references'])} |"
        )
    lines.append("")

    ignored_executables = fam["ignore_when"]["executables"]
    if ignored_executables:
        names = ", ".join(f"`{e}`" for e in ignored_executables)
        lines.append(
            f"Internal, not user-facing: {names}. Bear recognizes these "
            "only so it can filter them back out; they never produce a "
            "database entry."
        )
        lines.append("")

    ignored_flags = fam["ignore_when"]["flags"]
    if ignored_flags:
        flags = ", ".join(f"`{f}`" for f in ignored_flags)
        lines.append(
            f"An invocation is also ignored when its arguments include "
            f"{flags}: that is an internal frontend or codegen call, not "
            "a user-facing compile."
        )
        lines.append("")

    return lines


def render_wrapper_table(wrappers: list[dict]) -> list[str]:
    lines = [
        "### Compiler launchers",
        "",
        "| Executable name | Recognized as | Configuration `as:` value | "
        "Documentation |",
        "|---|---|---|---|",
    ]
    for fam in wrappers:
        entry = fam["recognize"][0]
        names = ", ".join(f"`{e}`" for e in entry["executables"])
        lines.append(
            f"| {names} | {entry['description']} | `{fam['id']}` | "
            f"{doc_links(entry['references'])} |"
        )
    lines.append("")
    return lines


def render_generated_block(compilers: list[dict], wrappers: list[dict]) -> str:
    lines = ["### Compiler families", ""]
    for fam in compilers:
        lines.extend(render_compiler_family(fam))
    lines.extend(render_wrapper_table(wrappers))
    # Drop the table's own trailing blank line so the block ends cleanly
    # regardless of which section rendered last.
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    compilers_dir = repo_root / "crates" / "semantic" / "compilers"
    template_path = repo_root / "site" / "src" / "reference" / "supported-compilers.md"

    if not compilers_dir.is_dir():
        print(f"error: no compilers directory at {compilers_dir}", file=sys.stderr)
        return 2
    if not template_path.is_file():
        print(f"error: no template page at {template_path}", file=sys.stderr)
        return 2

    output_path = Path(sys.argv[1]) if len(sys.argv) > 1 else template_path

    try:
        compilers, wrappers = load_families(compilers_dir)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    text = template_path.read_text()
    if BEGIN_MARKER not in text or END_MARKER not in text:
        print(
            f"error: {template_path} is missing the generated-block markers",
            file=sys.stderr,
        )
        return 1

    begin_idx = text.index(BEGIN_MARKER) + len(BEGIN_MARKER)
    end_idx = text.index(END_MARKER)
    if end_idx < begin_idx:
        print("error: END marker precedes BEGIN marker", file=sys.stderr)
        return 1

    generated = render_generated_block(compilers, wrappers)
    new_text = (
        text[:begin_idx]
        + "\n\n"
        + generated.rstrip("\n")
        + "\n\n"
        + text[end_idx:]
    )

    output_path.write_text(new_text)
    return 0


if __name__ == "__main__":
    sys.exit(main())

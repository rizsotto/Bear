#!/usr/bin/env python3
# Generate the enumerated tables on site/src/reference/command-line.md from
# the installed `bear` binary's own `--help` output -- the actual source of
# truth for flags, arguments, subcommands, and defaults, since clap (the
# argument parser Bear is built with) derives that text from the same
# definitions that parse the command line at runtime. This keeps the page
# from hand-copying (and silently drifting from) the man page's OPTIONS and
# COMMANDS sections, which are themselves written by hand.
#
# Locating the binary: there is no `target/debug/bear` to run directly --
# `bear` is a shell wrapper that `scripts/install.sh` generates around
# `bear-driver`, embedding the install prefix as a literal path (see
# site/CLAUDE.md, "Authoritative sources"). So this script never builds or
# installs anything itself; it expects a throwaway install to already exist,
# resolved in this order:
#
#   1. The BEAR_BIN environment variable, if set: an explicit path to a
#      `bear` executable.
#   2. /tmp/bear-review/bin/bear, the throwaway-install location already
#      documented in site/CLAUDE.md for exactly this purpose.
#
# If neither resolves to an executable file, the script FAILS LOUDLY with
# exit code 2 and a message naming the commands to run -- it never falls
# back to stale or partial output. A contributor sets this up with:
#
#     cargo build
#     SRCDIR=target/debug PREFIX=/tmp/bear-review INTERCEPT_LIBDIR=lib \
#         ./scripts/install.sh
#     python3 scripts/generate-command-line-reference.py
#
# Determinism: the rendered page is a pure function of the four `--help`
# texts (top level, intercept, semantic, parse-sh). No timestamp, hostname,
# or path from the machine that ran this script is written to the page, and
# the binary's resolved location itself never appears in the output. The
# version string is deliberately NOT embedded: `--version` output changes on
# every release, which would make this page diff on every release commit for
# no reader benefit; the page instead describes the current command-line
# shape, not a specific version of it.
#
# Parsing approach: clap's default help formatter has a small, fixed
# structure -- an optional one-line description, one or more "Usage:"
# lines, then "Commands:", "Arguments:", and "Options:" sections (each a
# list of "<name-or-flags>  <description>" lines, two-plus spaces apart),
# followed by optional trailing prose with an indented example. This script
# parses exactly that shape with a small line-oriented state machine; it
# does not implement a general clap-help or argparse-help parser.
#
# Run from anywhere:
#     python3 scripts/generate-command-line-reference.py
#
# Only the region between the BEGIN/END markers in
# site/src/reference/command-line.md is replaced; everything outside it
# (the page's hand-written explanatory prose) is read from the current file
# and carried through unchanged. An optional argument overrides the output
# path (used by scripts/check-docs-site.sh to render into a scratch file
# without touching the committed page):
#
#     python3 scripts/generate-command-line-reference.py /tmp/scratch.md
#
# Exit codes:
#   0 - the page was (re)generated
#   1 - --help output could not be parsed in the expected shape, or the
#       markers are missing from the template page
#   2 - invocation error (no usable bear binary found, or the template page
#       is missing)

import os
import re
import subprocess
import sys
from pathlib import Path

BEGIN_MARKER = (
    "<!-- BEGIN GENERATED: scripts/generate-command-line-reference.py -- "
    "DO NOT EDIT. Edits inside this block are lost the next time the "
    "script runs. -->"
)
END_MARKER = "<!-- END GENERATED -->"

DEFAULT_BINARY = Path("/tmp/bear-review/bin/bear")

# (subcommand, invocation shown in headings and usage)
SUBCOMMANDS = ["intercept", "semantic", "parse-sh"]

DEFAULT_RE = re.compile(r"\s*\[default:\s*(.*?)\]\s*$")
SECTION_HEADERS = {"Commands:", "Arguments:", "Options:"}


def resolve_binary() -> Path:
    """Find a usable `bear` executable, or exit loudly. Never returns a
    path that does not exist and is not executable."""
    env_bin = os.environ.get("BEAR_BIN")
    candidates = [Path(env_bin)] if env_bin else []
    candidates.append(DEFAULT_BINARY)

    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate

    print("error: no usable bear binary found", file=sys.stderr)
    print(
        "  checked: "
        + ", ".join(str(c) for c in candidates)
        + " (BEAR_BIN env var, then the default throwaway install)",
        file=sys.stderr,
    )
    print("hint: build and install a throwaway copy first:", file=sys.stderr)
    print("  cargo build", file=sys.stderr)
    print(
        "  SRCDIR=target/debug PREFIX=/tmp/bear-review INTERCEPT_LIBDIR=lib "
        "./scripts/install.sh",
        file=sys.stderr,
    )
    sys.exit(2)


def run_help(binary: Path, args: list[str]) -> str:
    result = subprocess.run(
        [str(binary), *args, "--help"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(
            f"error: `{binary} {' '.join(args)} --help` exited "
            f"{result.returncode}",
            file=sys.stderr,
        )
        print(result.stderr, file=sys.stderr, end="")
        sys.exit(1)
    return result.stdout


def split_two_spaces(text: str) -> tuple[str, str]:
    """Split a "<label>  <description>" help line on the first run of two
    or more spaces, the same convention clap uses to align its columns."""
    parts = re.split(r"\s{2,}", text.strip(), maxsplit=1)
    if len(parts) != 2:
        raise ValueError(f"cannot split into label/description: {text!r}")
    return parts[0].strip(), parts[1].strip()


def parse_help(text: str, context: str) -> dict:
    """Parse one --help output into a structured dict: description
    (str), usage (list[str]), commands/arguments/options (list of dict),
    trailing (list of ('prose'|'example', str) blocks)."""
    lines = text.splitlines()
    i = 0
    n = len(lines)

    def skip_blank():
        nonlocal i
        while i < n and not lines[i].strip():
            i += 1

    skip_blank()

    description_lines = []
    while i < n and not lines[i].startswith("Usage:"):
        if lines[i].strip():
            description_lines.append(lines[i].strip())
        i += 1
    if i >= n:
        raise ValueError(f"{context}: no 'Usage:' line found")
    description = " ".join(description_lines)

    usage = [lines[i][len("Usage:") :].strip()]
    i += 1
    while i < n and lines[i].strip() and lines[i][0].isspace():
        usage.append(lines[i].strip())
        i += 1

    commands: list[dict] = []
    arguments: list[dict] = []
    options: list[dict] = []
    section = None

    while i < n:
        stripped = lines[i].strip()
        if not stripped:
            i += 1
            continue
        if stripped in SECTION_HEADERS:
            section = stripped
            i += 1
            continue
        if not lines[i][0].isspace():
            # A column-0 line that is not a known section header: this is
            # the start of trailing prose, handled below.
            break
        label, desc = split_two_spaces(lines[i])
        if section == "Commands:":
            commands.append({"name": label, "description": desc})
        elif section == "Arguments:":
            arguments.append({"name": label, "description": desc})
        elif section == "Options:":
            m = DEFAULT_RE.search(desc)
            default = m.group(1) if m else None
            if m:
                desc = desc[: m.start()].rstrip()
            options.append({"flags": label, "description": desc, "default": default})
        else:
            raise ValueError(f"{context}: indented line outside a known section: {lines[i]!r}")
        i += 1

    trailing: list[tuple[str, str]] = []
    while i < n:
        skip_blank()
        if i >= n:
            break
        indented = lines[i][0].isspace()
        block_lines = []
        while i < n and lines[i].strip() and (lines[i][0].isspace() == indented):
            block_lines.append(lines[i].strip())
            i += 1
        kind = "example" if indented else "prose"
        block_lines_text = "\n".join(block_lines) if kind == "example" else " ".join(block_lines)
        trailing.append((kind, block_lines_text))

    return {
        "description": description,
        "usage": usage,
        "commands": commands,
        "arguments": arguments,
        "options": options,
        "trailing": trailing,
    }


def escape_cell(text: str) -> str:
    return text.replace("|", "\\|")


def render_options_table(options: list[dict]) -> list[str]:
    lines = ["| Flag | Description | Default |", "|---|---|---|"]
    for opt in options:
        default = f"`{opt['default']}`" if opt["default"] else "-"
        lines.append(
            f"| `{escape_cell(opt['flags'])}` | {escape_cell(opt['description'])} "
            f"| {default} |"
        )
    lines.append("")
    return lines


def render_arguments_table(arguments: list[dict]) -> list[str]:
    lines = ["| Argument | Description |", "|---|---|"]
    for arg in arguments:
        lines.append(f"| `{escape_cell(arg['name'])}` | {escape_cell(arg['description'])} |")
    lines.append("")
    return lines


def render_trailing(trailing: list[tuple[str, str]]) -> list[str]:
    lines = []
    for kind, block in trailing:
        if kind == "prose":
            lines.append(block)
            lines.append("")
        else:
            lines.append("```sh")
            lines.extend(block.splitlines())
            lines.append("```")
            lines.append("")
    return lines


def render_usage(usage: list[str]) -> list[str]:
    return ["```", *usage, "```", ""]


def render_top_level(parsed: dict) -> list[str]:
    lines = ["## Global usage", ""]
    lines.extend(render_usage(parsed["usage"]))
    lines.append(parsed["description"])
    lines.append("")
    if parsed["commands"]:
        lines.append("### Subcommands")
        lines.append("")
        lines.append("| Command | Description |")
        lines.append("|---|---|")
        for cmd in parsed["commands"]:
            lines.append(f"| `{escape_cell(cmd['name'])}` | {escape_cell(cmd['description'])} |")
        lines.append("")
    if parsed["arguments"]:
        lines.append("### Arguments")
        lines.append("")
        lines.extend(render_arguments_table(parsed["arguments"]))
    if parsed["options"]:
        lines.append("### Options")
        lines.append("")
        lines.extend(render_options_table(parsed["options"]))
    lines.extend(render_trailing(parsed["trailing"]))
    return lines


def render_subcommand(name: str, parsed: dict) -> list[str]:
    lines = [f"## bear {name}", ""]
    lines.append(parsed["description"])
    lines.append("")
    lines.extend(render_usage(parsed["usage"]))
    if parsed["arguments"]:
        lines.append("### Arguments")
        lines.append("")
        lines.extend(render_arguments_table(parsed["arguments"]))
    if parsed["options"]:
        lines.append("### Options")
        lines.append("")
        lines.extend(render_options_table(parsed["options"]))
    lines.extend(render_trailing(parsed["trailing"]))
    return lines


def render_generated_block(top: dict, subs: dict) -> str:
    lines = render_top_level(top)
    for name in SUBCOMMANDS:
        lines.extend(render_subcommand(name, subs[name]))
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    template_path = repo_root / "site" / "src" / "reference" / "command-line.md"

    if not template_path.is_file():
        print(f"error: no template page at {template_path}", file=sys.stderr)
        return 2

    output_path = Path(sys.argv[1]) if len(sys.argv) > 1 else template_path

    binary = resolve_binary()

    try:
        top = parse_help(run_help(binary, []), "bear")
        subs = {
            name: parse_help(run_help(binary, [name]), f"bear {name}")
            for name in SUBCOMMANDS
        }
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

    generated = render_generated_block(top, subs)
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

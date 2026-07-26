# CLAUDE.md - `docs/` guide

This is the index for project documentation - start here to find what
each part holds.

The two role-based subdirectories:

| Directory | Holds | Guide |
|---|---|---|
| [`requirements/`](requirements/) | Contracts: what Bear must do, from the user's perspective. Verified by tagged tests. | [`requirements/CLAUDE.md`](requirements/CLAUDE.md) |
| [`rationale/`](rationale/) | Decision records: the reasoning behind a design choice (or a rejected alternative). | [`rationale/CLAUDE.md`](rationale/CLAUDE.md) |

Keep the roles separate. A requirement says *what*; a rationale entry
says *why*; the code says *how*. Reasoning does not belong in a
requirement body, and a contract does not belong in a rationale entry.

The user-facing reference - CLI flags, the YAML configuration schema and
its keys, defaults, and examples - is the man page
[`../man/bear.1.md`](../man/bear.1.md), kept honest by the integration
tests. Literal config keys and flag names live there and in the code, not
in a requirement body.

The user-facing documentation site (the guide users read, published to
GitHub Pages) is a separate role and lives at repo-root
[`../site/`](../site/); its authoring rules are in
[`../site/CLAUDE.md`](../site/CLAUDE.md).

`docs/` holds reference documentation. Operational procedures (releasing,
and similar) are invocable skills under
[`../.claude/skills/`](../.claude/skills/), not files here.

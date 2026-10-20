## Requirements directory

Each file here is one contract: the *what* the user can expect from Bear.
Tests verify the implemented ones.

Does not belong here:

- Bear's own config keys, CLI flags, or schema fragments. Describe the
  behavior ("a configuration option toggles inlining"); the man page
  (`man/bear.1.md`) names the key. Flags of the compilers Bear classifies
  (`-c`, `--version`) are subject matter and do appear.
- Implementation guides, algorithm walkthroughs, error-handling tables.
- Design reasons, trade-offs, rejected alternatives.
- Bug reports and to-dos. Those go in the issue tracker.

## Files and IDs

A file is named `<area>-<short-name>.md`, for example `output-append.md`.
The name without `.md` is the requirement ID. Write it that way, in
backticks, in cross-references (`output-append`) and in test tags.

Fill [`TEMPLATE.md`](TEMPLATE.md) for the format. Tests cite IDs with the
`Requirements:` tag described in
[`../../tests/integration/CLAUDE.md`](../../tests/integration/CLAUDE.md).

## Status

| Status | Meaning |
|---|---|
| `proposed` | Drafted, waiting for the user's review |
| `accepted` | Approved by the user for implementation |
| `implemented` | Code complete, tagged tests passing |
| `rejected` | Declined by the user, or withdrawn |

Only the user sets `accepted` or `rejected`. When the user approves or
declines a requirement in the current session, the agent may set the
status for them. The agent sets `implemented` once the code is done and
the tagged tests pass. A changed contract goes back to `proposed`.

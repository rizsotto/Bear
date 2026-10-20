## Requirements directory

Each file here is one contract: the *what* the user can expect from Bear.
Tests verify the implemented ones.

Does not belong here:

- Bear's own CLI flags, config keys, config values, or schema fragments,
  in any section, acceptance criteria included. The ban is on the
  spelling the user types (`mode: wrapper`), not on the behavior it
  selects ("the preload mechanism is selected on Linux" is fine).
  Describe the behavior ("when appending", "a configuration option
  toggles inlining"); the man page (`man/bear.1.md`) names the flag or
  key. Flags of the compilers Bear classifies (`-c`, `--version`) are
  subject matter and do appear.
- Copies of lists that a compiler YAML in `crates/semantic/compilers/`
  owns, such as flag or compiler names. State the rule ("module-interface
  flags are recognized") and let the YAML own the set, as
  `recognition-compiler-names` does.
- Implementation guides, algorithm walkthroughs, error-handling tables.
- Design reasons, trade-offs, rejected alternatives.
- Bug reports and to-dos. Those go in the task tracker.

## Terms

These apply to prose. Existing IDs and titles that use an old term are
renamed in their area sweep, not ad hoc.

- Run modes are named as in `man/bear.1.md`: combined mode, intercept
  mode, semantic mode, parse-sh mode.
- Interception uses the preload mechanism or the wrapper mechanism, as in
  `interception-preload-mechanism` and `interception-wrapper-mechanism`,
  even where the man page and the config key say "mode".
  Never "preload mode", "wrapper mode", or "interception mode".
- Append is a behavior ("when appending"), not a mode.

## Files and IDs

A file is named `<area>-<short-name>.md`, for example `output-append.md`.
The name without `.md` is the requirement ID. Write it that way, in
backticks, in cross-references between requirement files
(`output-append`) and in test tags.

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

Whether a declined idea keeps a `rejected` file or becomes a rationale
entry is settled in [`../rationale/CLAUDE.md`](../rationale/CLAUDE.md).

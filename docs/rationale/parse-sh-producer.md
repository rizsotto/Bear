# parse-sh: a quarantined, best-effort parser whose only product is the database

## Context

Users have asked for years for a compilation database built from the
text a build system prints in dry-run mode (`make -n` and friends),
without paying for a full build. See
[#284](https://github.com/rizsotto/Bear/issues/284),
[#287](https://github.com/rizsotto/Bear/issues/287),
[#219](https://github.com/rizsotto/Bear/issues/219),
[#644](https://github.com/rizsotto/Bear/issues/644),
[#456](https://github.com/rizsotto/Bear/issues/456) and
[#404](https://github.com/rizsotto/Bear/issues/404). Every one of them
asks for `compile_commands.json`. None asks for an intermediate stream.
The prior art they cite, [compiledb](https://github.com/nickdiego/compiledb),
regex-scrapes `make -n` output straight into a database.

Bear once declined the closest form of this.
[#644](https://github.com/rizsotto/Bear/issues/644) asked for a
build-log post-processor and was rejected as build-system-specific. A
shell-text parser is still best-effort and build-system-flavored. What
changed is that Bear now has a clean seam: the semantic stage consumes
executions and does not care who produced them. A parser that yields
executions needs nothing else from the core.

Options considered:

- A flag on the interception path (`bear --dry-run`). Rejected: Bear
  cannot intercept without running the build. Interception observes real
  `exec()` calls, so the flag would misrepresent what the tool does.
- A monolithic parse-to-database tool like compiledb. Rejected: it
  would duplicate the semantic and output stack and tie the fragile
  parser to compiler recognition and formatting.
- Regex scraping. Rejected: regexes mis-handle quoting, `VAR=value`
  prefixes, redirections and separators. They produce plausible but wrong
  commands with no signal. A real lexer over an explicit subset can skip
  what it cannot parse, and say so.
- A separate executable. Rejected: `intercept` and `semantic` already
  set the pattern of stages as subcommands. A new binary adds install,
  man page and completions work for no gain.
- The name `dryrun`. Rejected as dishonest: the build system dry-runs,
  this tool only parses the text it printed. `parse-sh` names the verb
  and the input.
- Emit the execution-event stream and document a two-stage pipe,
  `make -n | bear parse-sh | bear semantic`. Rejected: it makes an
  implementer's seam the user interface. Bear's usual shape is one
  command in, database out (`bear -- make`).
- A flag that selects events or database output. Rejected: it would flip
  both the meaning and the default of the output destination, and make
  config validity depend on another flag.
- A sibling `parse-sh-events` subcommand, shown or hidden from help.
  Rejected: no user asked for the raw stream. A hidden one would also make
  help, man page and completions disagree.
- Keep the stream for caching or debugging. Rejected: an intercepted
  event file is worth keeping because a rebuild is expensive, but
  dry-run text is cheap to keep and parse again. Skip reports on stderr
  and debug logging already show what the parser saw.
- Support subshell groups `( ... )`. Rejected: a child shell's state must
  not leak back out, and modeling a child shell is the growth the skip
  path exists to avoid.

Adding a public events output later is a compatible change. Shipping one
and removing it is a breaking change. So the smaller surface wins.

## Decision

`parse-sh` is a `bear` subcommand. It lexes a small, explicit POSIX `sh`
subset from text (stdin by default) and writes `compile_commands.json`
(the default destination). Config, append and stdout output work as in
the other database modes. The reconstructed executions feed the
unchanged semantic stage inside the process and are never written out.
Text outside the subset is skipped with a line number and reason, never
guessed at. Interception stays the recommended high-fidelity path.

Inside the subset, make's `Entering directory` / `Leaving directory`
markers are recognized, since they would hit the skip path anyway and
they give correct directory tracking. Brace groups `{ ...; }` are
supported because they run in the current shell, so a `cd` inside one
persists.

## Consequences

- The headline usage is one stage: `make -n | bear parse-sh`.
- The [#644](https://github.com/rizsotto/Bear/issues/644) rejection is
  narrowed, not reversed. The parser sits behind the execution seam.
  The core stays parser-agnostic and can take other producers
  (xcodebuild logs, strace) without change.
- `bear intercept` is the only Bear producer of the event file. The
  format keeps its value for third-party producers feeding
  `bear semantic`.
- Fidelity is bounded by the dry-run text, by design. The best-effort
  framing must stay loud in user docs, or users will read a partial
  database as a Bear bug.
- Bear carries a shell lexer. It is small and closed, which caps the
  cost. Do not grow it toward a full shell. The skip path, not more
  grammar, is the pressure valve.
- Revisit the events output if real demand appears, for example link
  commands once linker support exists. It can be added compatibly.

## References

- [interception-shell-text-parsing](../requirements/interception-shell-text-parsing.md), the contract this supports
- [interception-events-format](../requirements/interception-events-format.md), the execution model the parser targets
- [parse-sh-single-tokenizer](parse-sh-single-tokenizer.md), how the lexer is built
- [event-file-defaults](event-file-defaults.md), the event-file defaults for `intercept` and `semantic`
- [#284](https://github.com/rizsotto/Bear/issues/284), the dry-run request; also [#287](https://github.com/rizsotto/Bear/issues/287), [#219](https://github.com/rizsotto/Bear/issues/219), [#644](https://github.com/rizsotto/Bear/issues/644), [#456](https://github.com/rizsotto/Bear/issues/456), [#404](https://github.com/rizsotto/Bear/issues/404)
- [compiledb](https://github.com/nickdiego/compiledb), the prior art

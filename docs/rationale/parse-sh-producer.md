# A quarantined best-effort producer for dry-run text

## Context

Users have asked for years (issues #284, #287, #219, #644, #456, #404)
for a compilation database built from the text a build system prints in
dry-run mode - `make -n` and friends - without paying for a full build.
The prior art they cite, `compiledb`, regex-scrapes `make -n` output
straight into a compilation database.

Bear had declined the closest form of this. Issue #644 asked for a
build-log post-processor and was rejected as build-system-specific and
out of scope. Two forces made that rejection worth revisiting:

- Bear already grew a clean internal seam. `intercept` writes a JSON
  Lines stream of execution events and `semantic --input` reads it; the
  [`interception-events-format`](../requirements/interception-events-format.md)
  requirement makes that stream a documented interchange contract. A
  producer that emits conforming events needs nothing from the core but
  the format.
- The demand is real and recurring, and the maintainer himself pointed at
  the execution-report path in #287 as the intended mechanism.

The tension: a shell/build-log parser is inherently best-effort and
build-system-flavored - exactly what #644 rejected - yet the request
keeps returning and the seam to satisfy it cleanly now exists.

Several shapes were on the table:

- A flag on the interception path (`bear --dry-run`). Rejected: Bear
  cannot intercept without executing; interception observes real
  `exec()` calls. A flag pretending to intercept a build that never ran
  would misrepresent what the tool does.
- A monolithic parse-to-cdb tool like `compiledb`. Rejected: it would
  duplicate the whole semantic and output stack that `semantic` already
  owns, and couple the fragile parser to compiler recognition and
  database formatting.
- Regex scraping of command lines. Rejected: regexes silently
  mis-handle quoting, `VAR=value` prefixes, redirections, and separators,
  producing plausible-but-wrong events with no signal that they are
  wrong. A real lexer over an explicit subset can instead skip what it
  cannot parse, loudly.
- A separate executable. Rejected: `intercept` and `semantic` already
  establish pipeline-stages-as-subcommands; a new binary would add
  install-layout, man-page, and completions plumbing for no benefit.
- Naming it `dryrun`. Rejected as dishonest: the build system dry-runs,
  this tool only parses the text it printed. The name `parse-sh` states
  the honest verb and names the input contract.

## Decision

Ship a producer, but quarantine it. It is a `bear` subcommand,
`parse-sh`, that lexes a limited, explicit POSIX `sh` subset from text
(stdin or file) and emits the documented execution-event stream (stdout
or file); everything downstream - compiler recognition, config, output,
dedup - is the unchanged `semantic` stage. The parser lives behind the
event seam and never touches the consumer half. Its contract,
[`interception-shell-text-parsing`](../requirements/interception-shell-text-parsing.md),
is explicitly best-effort: constructs outside the subset are skipped with
a line number and reason, never guessed at, and the dry-run non-guarantees
are written into both the requirement and the man page. Interception
stays the recommended high-fidelity default.

Two boundary calls inside the subset follow the same stance. The
recursive-make `Entering directory` / `Leaving directory` markers are
recognized as an explicit extension because they are free: without
recognition they would hit the skip path anyway, so admitting them
costs nothing and buys correct directory tracking. Brace groups
`{ ...; }` are supported because they run in the current shell, so a
`cd` inside one legitimately persists; subshell groups `( ... )` stay
unsupported because a child shell's state must not leak back out, and
modeling a child shell is exactly the growth the skip path exists to
avoid.

*Update (pre-4.2.0 release):* the event stream later became internal to
the mode. `parse-sh` now produces the compilation database directly and
no longer serializes events; see
[parse-sh-database-product](parse-sh-database-product.md). The
quarantine itself - a parser isolated behind the execution seam, the
semantic stage unchanged - stands.

This narrowly revises the #644 rejection. What was rejected was a
build-log parser wired into the core; what ships is a parser isolated to
one producer whose only output is the public event format, leaving the
core parser-agnostic and free to gain other producers (xcodebuild logs,
strace) on the same seam without change.

## Consequences

- The recurring request is answered without compromising interception's
  honesty or the core's simplicity.
- The event stream gained a second in-tree producer, validating the
  seam for future producers. (Weakened by the later database-product
  decision, which made the stream internal again; see
  [parse-sh-database-product](parse-sh-database-product.md).)
- Fidelity is bounded by dry-run text quality, permanently and by design.
  The best-effort framing must stay loud in user-facing docs, or users
  will read a partial database as a Bear bug rather than a dry-run limit.
- Bear now carries a shell lexer to maintain. It is deliberately small
  and closed (a fixed subset, skip-the-rest), which caps that cost; the
  temptation to grow it toward a full shell must be resisted - the skip
  path, not more grammar, is the pressure valve.

## References

- Requirements:
  [`interception-shell-text-parsing`](../requirements/interception-shell-text-parsing.md)
  (the producer contract) and
  [`interception-events-format`](../requirements/interception-events-format.md)
  (the seam it depends on).
- Issues: #284 (dry-run request), #287, #219, #644 (build-log parser,
  previously declined), #456, #404.
- Prior art: `compiledb` (nickdiego/compiledb), the regex-based tool this
  supersedes on Bear's seam.

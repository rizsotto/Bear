# A single streaming tokenizer for parse-sh

## Context

The first parse-sh implementation (the 4.2.0-rc branch) split lexical
knowledge across several independent scanners: the lexer's word reader
owned quotes, escapes, and comments; the interpreter re-scanned the raw
line with its own quote rules to detect here-documents and again to read
the here-doc delimiter; and a separate pre-pass joined backslash-newline
continuations, duplicating the lexer's own continuation stripping. A
three-pass review of the branch (2026-07-10) traced a whole bug class to
those scanners drifting apart: the here-doc detector did not understand
comments, so a `<<` inside a trailing `#` comment silently swallowed all
subsequent lines; its delimiter reader used a different word-break set
than the lexer, so a here-doc inside a subshell swallowed the rest of
the input; and a redirect-target reader dropped the unterminated-quote
signal the word reader had already computed, fabricating executions from
quoted text. Point fixes close each hole, but the shape guarantees more:
any two scanners over the same text will disagree eventually.

Alternatives on the table for a structural fix:

- Keep the multi-scanner shape, point-fix each divergence as found.
  Rejected: it treats symptoms; the next divergence is a matter of time,
  and each fix must be re-discovered by review rather than prevented.
- Rebuild the parser on a parser-combinator library (nom or winnow),
  producing an AST walked by a visitor to emit executions. Rejected on
  three grounds. First, combinators restructure the scanner but do not
  encode POSIX; the bugs found were missing grammar knowledge (the `>|`
  operator, comment boundaries) and would survive a mechanical rewrite.
  Second, shell is hostile to combinator parsing: a here-doc body binds
  at the next newline, not at the `<<` operator, forcing stateful
  two-phase lexing that fights the combinator model (real shells
  hand-write their lexers for this reason). Third, a full AST inverts
  the contract: the requirement defines a supported subset with loud
  skips and "never guesses"; certifying that property means auditing
  what the lexer recognizes, which is easy, not what a general parser
  accepts, which is not. It would also add a dependency against the
  workspace's minimal-dependency rule for no removed complexity.
- Adopt a complete POSIX shell parser crate (yash-syntax). Rejected for
  the same contract inversion, in stronger form: it parses all of shell,
  so every construct it accepts needs either semantics or a mapping back
  to a skip reason, and the deliberately small subset stops being small.
- One hand-written tokenizer owning every lexical rule, feeding a
  token-consuming parser that owns the interpreter state. Chosen.

## Decision

parse-sh is split into a single streaming tokenizer and a token-stream
parser: the tokenizer is the only code that reads characters (quotes and
escapes, comments, backslash-newline continuations, redirect operators
and their targets, here-documents, and the recursive-make directory
markers) and emits a small token set plus recoverable skip items for
unsupported constructs, while the parser consumes those tokens and owns
all interpreter state (working directory, the marker-driven directory
stack, and the positional `VAR=value` environment overlay).

Both stages are incremental state machines that hold at most one logical
line, so a skip anywhere on a line can still poison the whole line while
memory stays bounded by the longest line. Three invariants keep the
split sound: the tokenizer keeps scanning a line after a skip (so a later
`<<` on the same line is still seen) and the parser discards to the next
newline; quote state resets at every newline, matching the line-oriented
contract; and the tokenizer does no ambient I/O, surfacing read failures
in-band as the stream's last item so the caller cannot lose them.

## Consequences

- The reviewed bug class becomes unrepresentable rather than patched:
  there is no second scanner to drift. Comment, quote, and word-break
  rules exist in exactly one place.
- A skip cannot be silently dropped; `Result` forces every consumer to
  handle it, which is the loud-skip contract expressed in the type.
- Memory is bounded by the longest logical line for input of any size:
  a gigabyte build log streams through without being held, and events
  reach the consumer as they are recognized.
- The grammar is still hand-maintained. Missing POSIX knowledge (the
  `>|` class of bug) is not prevented by this shape, only made cheaper
  to add in one place. The skip path remains the pressure valve; the
  subset must not grow toward a full shell (see
  [parse-sh-producer](parse-sh-producer.md)).
- The refactor must land behind the tests that pin current behavior,
  including the review's point fixes, so equivalence is checked rather
  than assumed.

## References

- Requirement:
  [`interception-events-from-shell-text`](../requirements/interception-events-from-shell-text.md)
  (the contract whose loud-skip clause drives the design).
- Rationale: [parse-sh-producer](parse-sh-producer.md) (why the
  producer exists and stays quarantined; this entry governs only its
  internal shape).
- Considered and rejected crates: nom / winnow (parser combinators),
  yash-syntax (complete POSIX shell parser).

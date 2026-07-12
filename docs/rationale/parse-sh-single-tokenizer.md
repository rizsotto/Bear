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
parser. The tokenizer is the only code that understands characters:
quotes and escapes (words arrive fully assembled, `foo'bar'baz` is one
Word token), comments, backslash-newline continuations, redirect
operators and their targets (consumed whole, never tokens: a target is
discarded uninspected because the command's argv is fully known without
it and only the redirect path stays unresolved, so skipping would lose
real events from automake/cmake logs for nothing the database needs;
an unterminated quote or substitution in one still skips loudly, since
that misreads the line boundary itself), here-documents
(the pending-delimiter queue and body consumption are internal; the
body is discarded, not stored), and the recursive-make directory
markers (recognized at line start, emitted as a Marker token). It
consumes any buffered reader, one logical line at a time, and is exposed
as an iterator whose items are a Token, a Skip (line number and reason
for an unsupported construct -- a recoverable item, not stream
termination), or a fatal read error that ends the stream. The token set
is small: Word, Separator (`;`, `&&`, `||`, `|`, `&`), Newline, Marker.
A here-document is a skip reason, not a token.

The parser consumes tokens and owns all interpreter state: working
directory, the marker-driven directory stack, and the per-command
environment overlay (leading `VAR=value` classification is positional,
so it happens here, on plain Word tokens). It is itself an iterator of
events (an execution, or a skipped line), yielding each as soon as its
line completes -- one line of buffering, because a skip anywhere on a
line must still be able to poison the whole line. Both stages are
incremental state machines: nothing is accumulated across lines, so the
producer that forwards events into the mode layer's channel runs in
memory bounded by the longest logical line.

Three rules keep the split sound:

- The tokenizer never stops scanning a line after yielding a skip; the
  parser discards items up to the next Newline. Skipping in the
  tokenizer would blind it to a later `<<` on the same line, and the
  here-doc body would be lexed as commands.
- Quote state resets at every newline. Real shell lets a double quote
  span lines, but the contract is line-oriented: an unclosed quote at
  end of line is an UnterminatedQuote skip, matching pinned behavior.
- No ambient I/O: the tokenizer reads only the reader the mode layer
  hands it (never `std::env` or `std::fs`), and read failures surface
  in-band as the stream's last item, so the caller cannot lose them.

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

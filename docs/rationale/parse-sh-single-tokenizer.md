# A single streaming tokenizer for parse-sh

## Context

The first parse-sh (the 4.2.0-rc branch) split its lexical rules across
several scanners. The lexer's word reader owned quotes, escapes and
comments. The interpreter scanned the raw line again, with its own quote
rules, to find here-documents and to read their delimiters. A separate
pre-pass joined backslash-newline continuations, which the lexer also
did.

The scanners drifted apart, and each drift was a bug. The here-doc
detector did not know about comments, so a `<<` in a trailing `#`
comment swallowed every later line. Its delimiter reader broke words
differently from the lexer, so a here-doc inside a subshell swallowed
the rest of the input. A redirect-target reader dropped the
unterminated-quote signal the word reader had already found, and so
made up executions from quoted text. Any two scanners over the same
text will disagree in the end.

Options considered:

- Keep the scanners and fix each drift as it is found. Rejected: it
  treats symptoms, and each fix has to be found by review instead of
  being prevented.
- Rebuild on a parser-combinator crate (nom or winnow), producing an AST
  that a visitor turns into executions. Rejected for three reasons.
  Combinators change the scanner but do not add POSIX knowledge, and the
  bugs were missing grammar knowledge (the `>|` operator, comment
  bounds). Shell fights combinators: a here-doc body starts at the next
  newline, not at `<<`, which needs stateful two-phase lexing (real
  shells hand-write their lexers for this reason). And a full AST
  inverts the contract: the requirement is a small subset with loud
  skips that never guesses, and that is easy to audit in a lexer but
  hard in a general parser. It would also add a dependency that removes
  no complexity.
- Use a complete POSIX shell parser crate (yash-syntax). Rejected for
  the same contract inversion, only stronger. It parses all of shell, so
  every construct it accepts needs either semantics or a mapping to a
  skip reason, and the small subset stops being small.
- One hand-written tokenizer that owns every lexical rule, feeding a
  parser that owns the interpreter state. Chosen.

## Decision

The tokenizer is the only code that reads characters: quotes, escapes,
comments, continuations, redirections, here-documents and the
recursive-make directory markers. The parser reads its tokens and skip
items and owns all interpreter state: the working directory, the
directory stack, and the `VAR=value` overlay.

After a skip the tokenizer keeps scanning the line, so a later `<<` is
still seen, and the parser discards up to the next newline. Quote state
resets at every newline, to match the line-oriented contract.

## Consequences

- The drift bugs cannot happen, because there is no second scanner.
  Comment, quote and word-break rules live in one place.
- A skip cannot be dropped silently. Tokens come as a `Result`, so every
  consumer must handle it. This is the loud-skip contract in the type.
- Both stages are streaming state machines that hold at most one
  logical line. A gigabyte build log streams through, and events reach
  the consumer as each line completes.
- The grammar is still written by hand. Missing POSIX knowledge (the
  `>|` kind of bug) is not prevented, only cheaper to add in one place.
  The skip path stays the pressure valve, and the subset must not grow
  toward a full shell (see [parse-sh-producer](parse-sh-producer.md)).

## References

- [interception-shell-text-parsing](../requirements/interception-shell-text-parsing.md), the contract whose loud-skip clause drives the design
- [parse-sh-producer](parse-sh-producer.md), why the producer exists; this entry covers only its internal shape
- [nom](https://crates.io/crates/nom), [winnow](https://crates.io/crates/winnow) and [yash-syntax](https://crates.io/crates/yash-syntax), the rejected crates

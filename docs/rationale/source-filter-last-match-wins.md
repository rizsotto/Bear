# Source filter: last matching rule wins

## Context

Bear can include or exclude source files from the compilation database by
directory rules (`sources.directories`). The original feature request
([#261](https://github.com/rizsotto/Bear/issues/261)) raised the question of precedence: when a file matches
several rules with conflicting actions, which one decides?

Two policies were on the table:

- **Fixed precedence** - e.g. "exclude always wins". Simple to reason
  about, but rigid: a user cannot carve out an exception inside an
  excluded subtree (include `src`, exclude `src/test`, then re-include
  `src/test/integration`).
- **Order-based, last match wins** - rules are evaluated top to bottom and
  the last one that matches sets the action.

## Decision

Use order-based evaluation where the last matching rule wins, with the
default action being "include" when no rule matches.

## Consequences

Users get full control over precedence by ordering rules, including
exceptions-to-exceptions, which a fixed policy cannot express. A common
pattern is a `.` catch-all first, then more specific rules after it. The
cost is that precedence is now positional: reordering rules changes
behaviour, so the rule list is order-sensitive and the user owns getting
the order right. Matching itself is literal prefix matching
(`Path::starts_with`), so rule paths must use the same path format the
entries are written in
([output-path-format](../requirements/output-path-format.md)).

## References

- [output-source-directory-filter](../requirements/output-source-directory-filter.md), the requirement this supports
- [#261](https://github.com/rizsotto/Bear/issues/261), the feature request

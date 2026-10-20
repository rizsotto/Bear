# Response-file inlining is best effort

## Context

Bear inlines `@file` response files after the build has run. By then a
file may be gone, since response files often live in the build
directory and get cleaned. A response file may also reference another
one, and not every compiler documents that nesting.

Failing the whole database for one missing file was an option. It was
not chosen: the build succeeded, so the file was there when it ran. A
missing file at analysis time is a stale build artifact, not an error
in the build. Refusing nested references that a compiler does not
document was also an option. It was not chosen: the build Bear saw
already accepted them, or it would not have compiled.

## Decision

Follow nested references for every compiler, up to a fixed depth that
guards against cycles. Leave any `@file` that cannot be read, or that
is past the depth limit, literal in the entry and log a warning.

## Consequences

One bad response file costs one entry its inlined flags, never the
whole database. The depth limit is not configurable. Revisit it only if
a real build nests deeper than the limit.

## References

- [output-response-file-inlining](../requirements/output-response-file-inlining.md), the requirement this supports
- [#701](https://github.com/rizsotto/Bear/issues/701), the feature request

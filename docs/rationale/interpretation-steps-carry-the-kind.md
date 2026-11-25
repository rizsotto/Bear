# Interpretation steps carry the kind of work

## Context

One call can do more than one kind of work. `cc a.c -o app` compiles
`a.c` and then links the result. `cc -o app a.o b.o` only links. Later,
`ar rcs libfoo.a a.o b.o` will archive. Consumers need to ask "what
compile work is in this call" without guessing.

Options considered:

- Put the kind on the call, by tool name (compiler, linker, archiver).
  The compile-and-link call has two kinds, so one label is wrong.
- Put the kind on the call, by the last step. This hides the compile in
  `cc a.c -o app`.
- Put no kind anywhere and add an archiving pass to the compiler
  pipeline. `ar` is not a compiler pass, and adding it breaks the order
  of the passes.

## Decision

The kind sits on steps derived from the interpretation, not on the call.
`cc a.c -o app` has a compile step and a link step. Consumers read steps
and do not match on how they were derived.

## Consequences

Entries come from compile steps, so a link-only call gives none. An
archiver has no passes, so it will need its own private rule for how its
steps are derived. That rule stays inside the interpretation.

## References

- [output-compilation-entries](../requirements/output-compilation-entries.md), the requirement this supports

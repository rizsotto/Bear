# Recognition keeps the execution

## Context

Recognition reads an intercepted call and says what it is. The output
layer then builds entries from the result. If the result replaces the
execution, the observed call is lost: an ignored call cannot show what it
was, and a recognized one cannot show what ran when the entry names
something else.

A compiler launcher such as ccache or distcc adds a wrinkle. In
`ccache gcc -c a.c` the launcher is part of the call. The entry must not
carry it, because the entry describes the compile.

Options considered:

- Borrow from the execution. A value cannot borrow from its own field,
  so a result that holds the execution and borrows from it does not
  compile. A split version works, where the caller owns the execution and
  the result borrows it. But that result cannot leave the scope that owns
  the execution, and the writer takes results as a lazy stream. Fixing
  that means collecting everything first, or a new dependency. Also, a
  value is not always a whole argv token. In `-omain.o` the path is a
  piece of one token.
- Unwrap the launcher before recognition. This mixes recognition with
  entry formatting. The launcher is a fact about the call, so recognition
  should keep it.

## Decision

Every outcome (recognized, ignored, not recognized) carries the
execution as observed. A recognized call adds an interpretation. The
interpretation keeps a launcher as its own item, and the entry drops it.
All strings are owned.

## Consequences

Recognition and entry formatting stay apart. Logs and tools can see the
call as it ran, whatever the outcome. Owned strings cost some copies, and
streaming to the writer keeps working.

## References

- [recognition-compiler-launchers](../requirements/recognition-compiler-launchers.md), the requirement this supports

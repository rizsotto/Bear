# Preload chains to co-resident preload libraries

## Context

A build can already carry another preload library, such as Gentoo's
`libsandbox.so`. That library may hook the same `exec` family as Bear's
and may reset its own preload variable further down the exec chain,
dropping Bear's entry. Grandchildren past that point are then not
intercepted ([issue #699](https://github.com/rizsotto/Bear/issues/699)).

Bear could guard its entry by refusing to delegate to the other
library. That would turn off the sandbox and change how the build
behaves, which interception must not do.

## Decision

Bear puts its library first in the preload variable, keeps the other
libraries after it, and forwards every call to the next library in the
chain.

## Consequences

The sandbox keeps working and the build behaves as without Bear. The
cost is that a library which resets the preload variable can hide part
of the build from Bear. Bear accepts that gap rather than break the
other library.

## References

- [interception-preload-mechanism](../requirements/interception-preload-mechanism.md), the requirement this supports
- [#699](https://github.com/rizsotto/Bear/issues/699), the sandbox conflict report

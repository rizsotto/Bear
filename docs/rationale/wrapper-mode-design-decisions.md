# Wrapper mechanism: hard links, deterministic directory, resolve-at-startup

## Context

The wrapper mechanism intercepts compilers by placing wrapper
executables, named after the compilers, ahead of the real ones on PATH.
Each wrapper reports the call and then runs the real compiler. Three
setup questions follow, and each has a force that is not obvious from
the code.

How a wrapper file is made. Tools like ccache detect symlinks to
themselves and skip them, but do not detect hard links. A symlink to
`bear-wrapper` was rejected for that reason: ccache could skip Bear's
wrapper entirely.

Where the wrapper directory lives. Autotools-style builds record the
compiler path during `./configure` and use it again during `make`. A
random temporary directory was rejected:
[#654](https://github.com/rizsotto/Bear/issues/654) showed that it
breaks such multi-step builds, because the recorded path can point at a
directory that a later step does not have.

When a bare compiler name becomes a path. The wrapper must know which
real compiler to run. Looking the name up on PATH in every wrapper call
was rejected: it repeats the lookup for each compiler call, and the
wrapper would depend on the PATH it runs with instead of on a
self-contained configuration.

## Decision

- Wrappers are **hard links** to `bear-wrapper`, not symlinks.
- The wrapper directory is a **deterministic** `.bear/` in the current
  working directory, not a random temp dir.
- Bare compiler names are **resolved to absolute paths once at startup**,
  not on every wrapper invocation.

## Consequences

- **Hard links, not symlinks.** ccache does not skip Bear's wrapper.
  The cost is the ccache recursion problem, handled separately in
  [interception-wrapper-recursion](../requirements/interception-wrapper-recursion.md).
- **Deterministic directory.** Paths recorded during `./configure` stay
  valid during `make`, provided both run under the same Bear invocation.
- **Resolve at startup.** No repeated PATH lookups, and the wrapper
  configuration is self-contained. The trade-off is that a PATH change
  mid-build is not picked up; the wrapper keeps using the path resolved
  at startup.

## References

- [interception-wrapper-mechanism](../requirements/interception-wrapper-mechanism.md), the requirement this supports
- [interception-wrapper-recursion](../requirements/interception-wrapper-recursion.md), the ccache recursion this choice causes
- [wrapper-recursion-ccache-alternatives](wrapper-recursion-ccache-alternatives.md), how the recursion is fixed
- [#654](https://github.com/rizsotto/Bear/issues/654), temp directories break multi-step builds

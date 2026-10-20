# Wrapper mechanism: hard links, deterministic directory, resolve-at-startup

## Context

The wrapper mechanism intercepts compilers by placing wrapper
executables, named after the compilers, ahead of the real ones on PATH.
Each wrapper reports the call and then runs the real compiler. Three
setup questions follow, and each has a force that is not obvious from
the code.

How a wrapper file is made. The wrapper takes the compiler name from
the file name of its own executable, and reads `wrappers.cfg` from the
directory that holds it. It gets that path from the OS, and on Linux the
OS resolves symlinks. A symlink to `bear-wrapper` was rejected for that
reason: the wrapper would see itself as `bear-wrapper` in the install
directory, find neither its compiler name nor its config, and fail. On
Windows the wrapper file is a copy.

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

- Wrappers are **hard links** to `bear-wrapper`, not symlinks. On
  Windows they are copies, and elsewhere a copy is the fallback when the
  hard link fails.
- The wrapper directory is a **deterministic** `.bear/` in the current
  working directory, not a random temp dir.
- Bare compiler names are **resolved to absolute paths once at startup**,
  not on every wrapper invocation.

## Consequences

- **Hard links, not symlinks.** Each wrapper sees its own name and its
  own directory, so it finds its compiler and its config. A copy works
  the same way, and covers Windows and file systems where hard links
  fail, such as overlay file systems in containers. The cost of a copy
  is disk space. The link type does not fix the ccache recursion
  problem: ccache does not recognize Bear's wrapper as itself. That is
  handled in
  [interception-wrapper-recursion](../requirements/interception-wrapper-recursion.md).
- **Deterministic directory.** Paths recorded during `./configure` stay
  valid during `make`, provided both run under the same Bear invocation.
- **Resolve at startup.** No repeated PATH lookups, and the wrapper
  configuration is self-contained. The trade-off is that a PATH change
  mid-build is not picked up; the wrapper keeps using the path resolved
  at startup.

## References

- [interception-wrapper-mechanism](../requirements/interception-wrapper-mechanism.md), the requirement this supports
- [interception-wrapper-recursion](../requirements/interception-wrapper-recursion.md), the ccache recursion the wrapper mechanism must avoid
- [wrapper-recursion-ccache-alternatives](wrapper-recursion-ccache-alternatives.md), how the recursion is fixed
- [#654](https://github.com/rizsotto/Bear/issues/654), temp directories break multi-step builds

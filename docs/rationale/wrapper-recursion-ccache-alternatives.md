# Wrapper recursion: why filter Bear's lookup PATH, not the alternatives

## Context

In wrapper mode, a compiler env var or PATH discovery can resolve to a
masquerade wrapper (ccache, distcc, icecc, ...) rather than the real
compiler. If Bear records that path, the build loops the wrapper into
itself. Several fixes were considered before settling on "resolve past
the masquerade dir and record the real compiler".

How the loop arises: masquerade wrappers (ccache, distcc, icecream,
colorgcc, buildcache) install a directory of symlinks named after real
compilers and prepend it to PATH; a bare `gcc` then resolves to the
wrapper, which looks up the real compiler further along PATH (skipping
its own symlinks) and forwards the call. Bear's wrapper mode prepends
its own directory of compiler-named links to PATH as well. A wrapper
that recognises itself only by symlink comparison cannot tell Bear's
link from a real compiler, selects it as the "real" `gcc`, and the two
forward to each other without end. distcc avoids that specific loop by
stripping every PATH entry up to and including its own directory, but
that also drops Bear's directory, silently breaking nested
interception.

## Decision

Filter Bear's own lookup PATH so resolution lands on the real compiler
past any masquerade directory, and record that path. Do not touch the
child's PATH or set ccache-specific variables.

## Consequences

Detecting a masquerade wrapper by resolving the symlink of the
discovered path, rather than matching a list of known directory
locations, covers distribution-local masquerade setups automatically:
any installer that symlinks compiler names to a wrapper binary is
caught, with no per-distro path table to maintain. The cost is missing
a masquerade installed as a shell script or hard copy, which no major
distro ships.

The rejected alternatives were each narrower or more fragile:

- **Set `CCACHE_COMPILER` in the wrapper's child environment** (the
  original proposal). The path the wrapper knows IS the ccache symlink
  (that is what `which gcc` returned at setup), and `CCACHE_COMPILER`
  pointing at a symlink-to-ccache makes ccache recurse into itself.
  Empirically verified on Fedora:
  `CCACHE_COMPILER=/usr/lib64/ccache/gcc ccache gcc -c foo.c` hangs and
  must be killed; `CCACHE_COMPILER=/usr/bin/gcc` works. Fixing it would
  require resolving past ccache to get the real path anyway - which is
  what this requirement does, making `CCACHE_COMPILER` redundant. It is
  also ccache-specific and would not help icecc or distcc.
- **`CCACHE_PATH` set to PATH minus `.bear/`.** ccache-specific (no
  equivalent for other wrappers), still requires enumerating a safe
  PATH, and does not address Bear's config pointing at the wrong
  executable.
- **Removing masquerade directories from the child's PATH.** Heavy-
  handed: a masquerade dir might also hold binaries that do not loop
  (some installs put `distcc` itself there); stripping it globally would
  break those. Filtering Bear's own lookup PATH is the narrower
  intervention.

## References

- Requirement: `interception-wrapper-recursion`
- Related requirement: `interception-wrapper-mechanism`
- Issue #445 - original PATH-ordering report
- Issue #686 - bare-name CC resolution
- ccache 4.x manual: https://ccache.dev/manual/4.10.2.html
- icecream masquerade setup: https://github.com/icecc/icecream

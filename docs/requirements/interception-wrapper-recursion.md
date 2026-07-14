---
title: Resolve past masquerade wrappers in wrapper mode
status: implemented
---

## Intent

When the user runs `bear -- make` on a distribution that ships compiler
masquerade wrappers (ccache on Fedora/Arch/Gentoo, icecream on its
supported distros, etc.), Bear's wrapper mode must not enter an infinite
loop with the masquerade wrapper. The compilation database must record
the real compiler command, and the build must complete. The user should
not have to strip any directories from PATH to make Bear work.

Bear achieves this by resolving past masquerade directories at
discovery time. The price is that while Bear is observing the build,
tools like ccache are not exercised -- the build sees the real compiler
directly. This is intentional: Bear observes, it does not optimise.

## Background: how masquerade wrappers break Bear

Compiler masquerade wrappers (ccache, distcc, icecream, colorgcc,
buildcache) install a directory of symlinks named after real compilers
and prepend it to PATH; a bare `gcc` then resolves to the wrapper, which
looks up the real compiler further along PATH (skipping its own
symlinks) and forwards the call. Bear's wrapper mode prepends its own
directory of compiler-named links to PATH as well. A wrapper that
recognises itself only by symlink comparison cannot tell Bear's link
from a real compiler, selects it as the "real" `gcc`, and the two
forward to each other without end. distcc avoids that specific loop by
stripping every PATH entry up to and including its own directory, but
that also drops Bear's directory, silently breaking nested interception.

Detection is by symlink resolution rather than by matching known
directory paths, so distribution-local masquerade setups are covered as
long as their installer symlinks compiler names to a wrapper binary.

## Acceptance criteria

- Wrapper mode completes without hanging when any supported masquerade
  wrapper directory is present in PATH
- The compilation database contains one entry per compiled source file
- The compiler path recorded in each entry is an absolute path to the
  real compiler, never the masquerade wrapper and never a `.bear/`
  wrapper
- The user is not required to strip any directory from PATH, unset
  any environment variable, or configure `CCACHE_*` manually
- If every `gcc` on PATH is a masquerade wrapper and no real compiler
  can be found past them, Bear reports a diagnostic and skips
  registering that compiler (it does not fall back to the wrapper)

## Non-functional constraints

- Detection must be pure filesystem inspection. No subprocess may be
  spawned to identify a wrapper (cost, trust).
- Resolution failure for one compiler must not fail Bear overall;
  other compilers are still registered.
- The set of recognised wrapper names is fixed in source. Uncommon
  or locally built wrappers that do not match are not detected; the
  user can either unset them from PATH or use preload mode.
- Detection is symlink-based. A masquerade wrapper installed as a
  shell script or hard copy (rather than a symlink) is out of scope
  and will not be detected. All major distros (Debian/Ubuntu,
  Fedora, Arch, Gentoo, macOS Homebrew) ship masquerade dirs as
  directories of symlinks, so this is a theoretical gap. If a
  non-symlink masquerade does surface in the wild, extend detection
  rather than widening the classification helper to read file
  contents.

## Testing

Given a host where `/usr/lib64/ccache/gcc -> /usr/bin/ccache` is first
in PATH:

> When the user runs `bear -- make` in wrapper mode,
> then the build completes within a normal timeout,
> and `compile_commands.json` contains one entry per source,
> and the recorded compiler path is an absolute path that is not
> a masquerade wrapper and not the Bear wrapper.

Given a host with no masquerade wrapper installed:

> When the user runs `bear -- make`,
> then Bear's resolution behaves identically to before (no filtering
> kicks in, no performance regression),
> and the compilation database is produced normally.

Given a compiler that exists only as a masquerade symlink on PATH
(no real compiler past it):

> When Bear resolves it,
> then Bear logs a warning naming the compiler and the masquerade
> dir(s) it excluded,
> and does not register a `.bear/` wrapper for it,
> and the build uses the compiler directly without Bear interception
> for that name.

Nested compiler invocations (a compiler driver spawning another
bare-name compiler in a grandchild process) must still be
intercepted; that guarantee is not specific to masquerade handling
and is covered by `interception-wrapper-mechanism`. This
requirement preserves it by not modifying the child's PATH.

## Notes

- Issue #445 -- original PATH-ordering report.
- Issue #686 -- bare-name CC resolution
  (`wrapper_mode_resolves_cc_bare_name_via_path`).
- Related requirement: `interception-wrapper-mechanism`.

## Rationale

- [Filter Bear's lookup PATH, not the alternatives](../rationale/wrapper-recursion-ccache-alternatives.md) -
  why `CCACHE_COMPILER`, `CCACHE_PATH`, and PATH-stripping were rejected.

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
- Detection is based on resolving the invoked path's symlink, not on
  matching known directory locations, so distribution-local masquerade
  setups are covered as long as the installer symlinks compiler names
  to a wrapper binary.
- Resolution failure for one compiler must not fail Bear overall;
  other compilers are still registered.

## Known limitations

**Fixed set of recognised wrapper names**: uncommon or locally built
masquerade wrappers whose names are not in the recognised set are not
detected -- kept fixed so detection stays a cheap, trusted filesystem
check. The user can unset such wrappers from PATH or use preload mode.

**Symlink-only detection**: a masquerade wrapper installed as a shell
script or hard copy (rather than a symlink) is not detected -- accepted
because all major distros (Debian/Ubuntu, Fedora, Arch, Gentoo, macOS
Homebrew) ship masquerade dirs as directories of symlinks, so this is
a theoretical gap.

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

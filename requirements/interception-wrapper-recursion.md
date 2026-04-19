---
title: Prevent wrapper recursion with compiler wrappers
status: proposed
tests: []
---

## Problem

When ccache is in PATH (the most common compiler wrapper setup), Bear's
wrapper mode can enter an infinite recursion loop:

1. Bear creates a wrapper hard link `.bear/gcc` -> `bear-wrapper`
2. Bear prepends `.bear/` to PATH
3. Build runs `gcc foo.c`
4. Shell finds `.bear/gcc` first in PATH (the Bear wrapper)
5. Bear wrapper reports the execution and invokes the "real" compiler
6. The "real" compiler is `/usr/lib64/ccache/gcc` (ccache's symlink)
7. ccache searches PATH for `gcc`, skipping only symlinks to itself
8. ccache finds `.bear/gcc` -- a hard link to `bear-wrapper`, NOT a
   symlink to ccache, so ccache accepts it as the real compiler
9. ccache executes `.bear/gcc`, which is Bear's wrapper again
10. Infinite loop: steps 5-9 repeat

This was observed during integration testing on Fedora where gcc is
symlinked through ccache (`/usr/lib64/ccache/gcc` -> `/usr/bin/ccache`).

The current workaround in the integration test (`intercept.rs:551-556`)
manually strips ccache directories from PATH before running Bear. This
is not available to end users.

## How compiler wrappers find the real compiler

Research into the major compiler wrappers (verified against their docs
and source code):

### ccache

**Source**: ccache manual (https://ccache.dev/manual/latest.html),
source `find_executable_in_path`.

- Searches the full PATH for the first executable matching the compiler
  name that is **not a symbolic link to ccache itself**.
- Detection uses `S_ISLNK` check + basename comparison to "ccache".
  Hard links and copies are NOT detected as ccache.
- **Env vars for real compiler**:
  - `CCACHE_COMPILER` (preferred) -- forces the compiler path, bypasses
    PATH search entirely.
  - `CCACHE_CC` -- deprecated alias for `CCACHE_COMPILER`.
  - `CCACHE_PATH` -- restricts which directories ccache searches for the
    compiler (colon-separated on Unix, semicolon on Windows).
- ccache does NOT read the `CC` or `CXX` env vars itself.

### distcc

**Source**: distcc(1) man page (https://www.distcc.org/man/distcc_1.html),
source `src/climasq.c`.

- In masquerade mode, strips **all directories up to and including** its
  own masquerade directory from PATH, then searches the remainder.
- This means if PATH is `.bear:/usr/lib/distcc/bin:/usr/bin`, distcc
  strips everything up to `/usr/lib/distcc/bin`, leaving only `/usr/bin`.
  Bear's `.bear/` is removed in this process.
- Self-detection uses string comparison on directory paths, not symlink
  resolution or inode comparison. Has a FIXME in source acknowledging
  this limitation.
- **No env var for real compiler**. The documented env vars are
  `DISTCC_HOSTS`, `DISTCC_LOG`, `DISTCC_VERBOSE`, `DISTCC_DIR`,
  `DISTCC_SSH` -- none for compiler override.
- **distcc does NOT cause recursion with Bear** because its aggressive
  PATH stripping removes `.bear/` along with everything before it.

### colorgcc

**Source**: colorgcc source (`colorgcc.pl` on GitHub).

- Reads compiler paths from `~/.colorgccrc` config file.
- Falls back to PATH search using `abs_path($0)` to skip entries that
  resolve to itself. Detects symlinks but NOT hard links.
- **No env var for real compiler**. The only env var check is
  `GCC_COLORS` -- if set, colorgcc skips colorization and execs the
  compiler directly.

### sccache

**Source**: sccache GitHub (https://github.com/mozilla/sccache).

- Direct invocation only (`sccache gcc -c foo.c`). No masquerade mode.
- **No env var for real compiler** (beyond RUSTC_WRAPPER for Rust).
- **sccache does NOT cause recursion with Bear** because it does not
  use symlink-in-PATH masquerade.

### Summary

| Tool | Causes recursion with Bear? | Env var for real compiler |
|---|---|---|
| ccache | **Yes** | `CCACHE_COMPILER` |
| distcc | No (strips all preceding PATH dirs) | None |
| colorgcc | Possible (hard link not detected) | None (config file only) |
| sccache | No (no masquerade mode) | None |

The primary problem is **ccache**, which is also by far the most common
compiler wrapper (default on Fedora, Arch, Gentoo, and others).

## Example scenario (ccache recursion)

System setup:
```
/usr/lib64/ccache/gcc -> /usr/bin/ccache
PATH=/usr/lib64/ccache:/usr/bin
```

User runs: `bear -- make`

Bear creates `.bear/gcc` (hard link to `bear-wrapper`) and sets
`PATH=.bear:/usr/lib64/ccache:/usr/bin`.

Trace:
1. Shell finds `.bear/gcc`, executes Bear wrapper
2. Bear wrapper looks up config: real compiler = `/usr/lib64/ccache/gcc`
3. Bear wrapper spawns `/usr/lib64/ccache/gcc foo.c` with PATH unchanged
4. ccache searches PATH for `gcc`, skips `/usr/lib64/ccache/gcc` (symlink
   to itself), but accepts `.bear/gcc` (hard link, not symlink to ccache)
5. ccache runs `.bear/gcc foo.c` -- back to step 1, infinite loop

## Integration test plan

The goal is to verify Bear works on a machine configured the way a user's
distribution ships it. Fedora, Arch, and Gentoo install ccache symlinks
in a masquerade directory and put that directory on PATH by default, so
when the user types `gcc` they are actually running ccache. We want to
confirm Bear does not loop in that exact setup.

Detection (in `build.rs`):
- Probe whether the host compiler resolved by the existing
  `compiler_c` check (`gcc`, `clang`, or `cc`) is actually a symlink
  pointing at a `ccache` binary. Walk the symlink chain and compare the
  target's filename to `ccache`.
- If it is, set a `cfg(host_compiler_goes_through_ccache)` flag and
  expose the resolved ccache binary path through an env var, the same
  way existing probes expose executables.
- If the host is not configured with ccache-in-PATH, the test is simply
  skipped via `#[cfg(host_compiler_goes_through_ccache)]`. We do not
  fabricate a masquerade directory -- synthetic ccache setups prove
  nothing about real user environments.

Set up (in the test):
- A test environment with a single source file `test.c`.
- An isolated `CCACHE_DIR` inside the test temp area so the test does
  not pollute the developer's real ccache cache and does not get flaky
  results from pre-existing cached entries.
- A build script that compiles `test.c` by invoking the compiler via
  its bare name, so PATH resolution kicks in exactly as it does in a
  real build. PATH itself is **not** modified -- we want the host's
  default PATH, ccache symlinks included.

Run Bear in wrapper mode against this build script.

Verify:
- The command completes within a reasonable timeout (no infinite loop).
- The exit status is success.
- The output compilation database contains exactly one entry for `test.c`.
- The compiler command recorded in the entry resolves to a real compiler
  binary, not Bear's wrapper.

Why this exercises the bug:
- The real ccache from the host distribution is in play, using its
  actual PATH-search logic (which accepts hard links as "not itself").
- Bear's `.bear/` directory is present in PATH because Bear puts it
  there in wrapper mode. Without the fix, ccache searches PATH, finds
  `.bear/gcc`, and loops.
- The fix (setting `CCACHE_COMPILER` in the wrapper's child environment)
  should route ccache directly to the real compiler, bypassing PATH
  search entirely.

Negative check (manual, not automated):
- Temporarily revert the fix and confirm the test hangs or times out,
  ensuring the test actually covers the bug rather than coincidentally
  passing.

## Acceptance criteria

- [ ] Wrapper mode completes without hanging when ccache symlinks are in PATH
- [ ] The compilation database is generated correctly
- [ ] No special user-side workarounds required (no manual PATH stripping)
- [ ] Nested compiler invocations are still intercepted (`.bear/` stays in PATH)
- [ ] User's existing `CCACHE_COMPILER` setting is not overridden

## Solution: Set `CCACHE_COMPILER` in the wrapper's child environment

The wrapper binary already knows the real compiler path (from the config
mapping). Before spawning the real compiler, set `CCACHE_COMPILER` to
that path. This tells ccache exactly which compiler to use, bypassing
its PATH search entirely.

This is the documented mechanism for controlling ccache's compiler
selection. It does not require removing `.bear/` from PATH, so nested
compilations remain intercepted.

Changes in `bear/src/bin/wrapper.rs`, after resolving the real executable:

```rust
// Tell ccache to use the real compiler directly, bypassing PATH search.
// This prevents ccache from finding Bear's wrapper in .bear/ and looping.
// Only set if the user hasn't already configured CCACHE_COMPILER.
if !execution.environment.contains_key("CCACHE_COMPILER") {
    execution.environment.insert(
        "CCACHE_COMPILER".into(),
        real_executable.to_string_lossy().into(),
    );
}
```

**Complexity**: Very low. ~5 lines in `wrapper.rs`.
**Alignment**: Good. Uses ccache's own documented interface. The wrapper
already has all the information needed (real compiler path from config).

**Why this is sufficient**:
- ccache is the only common wrapper that causes recursion with Bear.
- distcc does not loop (its PATH stripping removes `.bear/`).
- colorgcc is rare and typically configured via `~/.colorgccrc` with
  explicit paths, which avoids the PATH search problem.
- Setting `CCACHE_COMPILER` when ccache is not involved is harmless --
  the variable is simply ignored by non-ccache compilers.
- The `contains_key` check preserves any user-configured value.

## Notes

- The integration test for issue #686 (`wrapper_mode_resolves_cc_bare_name_via_path`)
  had to manually strip ccache from PATH to work. With this fix, that
  workaround could be removed.
- ccache documentation recommends placing its directory first in PATH, which
  is the standard setup on Fedora, Arch, Gentoo, and other distributions.
- `CCACHE_CC` is a deprecated alias for `CCACHE_COMPILER`. We should use the
  modern name.
- ccache 4.x documentation: https://ccache.dev/manual/4.10.2.html
- Related issue: #686.

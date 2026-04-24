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

Compiler masquerade wrappers (ccache, distcc, icecream/icecc,
colorgcc, buildcache) install a directory of symlinks named after real
compilers (`/usr/lib64/ccache/gcc`, `/usr/lib/icecc/bin/gcc`, ...) where
each symlink points at the wrapper binary. The distribution prepends
that directory to PATH, so a bare `gcc` in a Makefile resolves to the
wrapper, which then looks up the real compiler on PATH (skipping its
own symlinks) and forwards the call.

Bear's wrapper mode puts `.bear/` (full of hard links to `bear-wrapper`)
at the front of PATH. On a ccache-equipped box the interaction is:

1. Shell finds `.bear/gcc`, runs Bear wrapper.
2. Wrapper reads its config: real `gcc` is `/usr/lib64/ccache/gcc` --
   whatever `which gcc` returned at Bear startup.
3. Wrapper execs `/usr/lib64/ccache/gcc` (which IS ccache).
4. ccache searches PATH for `gcc`, skipping symlinks to itself. It
   does NOT skip `.bear/gcc` because that is a hard link, not a
   symlink, so ccache accepts it as the real compiler.
5. ccache execs `.bear/gcc`, Bear wrapper runs again. Steps 2-5
   repeat forever.

The same shape applies to any masquerade wrapper that detects itself
only by symlink comparison. distcc in masquerade mode happens to avoid
this specific loop because it strips all PATH entries up to and
including its own dir -- which drops `.bear/` as collateral damage --
but that still means distcc silently removes Bear from the child's
PATH, which breaks nested interception even when no loop occurs.

### Known masquerade wrappers

| Tool                 | Masquerade dir examples                      | Notes                                                                |
|----------------------|----------------------------------------------|----------------------------------------------------------------------|
| ccache               | `/usr/lib64/ccache`, `/usr/lib/ccache`       | Default on Fedora, Arch, Gentoo. Loops with Bear.                    |
| distcc               | `/usr/lib/distcc`, `/usr/lib/distcc/bin`     | Strips PATH prefix including `.bear/`; no loop, but breaks nesting.  |
| icecream / icecc     | `/usr/lib/icecc/bin`, `/usr/libexec/icecc`   | Symlink pattern same as ccache. Loops with Bear.                     |
| colorgcc             | `~/bin/colorgcc` setups                      | Rare; typically configured via `~/.colorgccrc`, not PATH masquerade. |
| buildcache           | `/usr/lib/buildcache/bin` (varies)           | Same shape as ccache.                                                |
| sccache              | Not a masquerade wrapper                     | Invoked explicitly (`sccache gcc ...`); no recursion with Bear.      |

Detection is by symlink resolution, not by matching directory paths,
so new or distribution-local masquerade setups are covered as long as
their installer symlinks compiler names to a wrapper binary.

## Acceptance criteria

- Wrapper mode completes without hanging when any supported masquerade
  wrapper directory is present in PATH
- The compilation database contains one entry per compiled source file
- The compiler path recorded in each entry is an absolute path to the
  real compiler, never the masquerade wrapper and never a `.bear/`
  wrapper
- Nested compiler invocations (a compiler driver spawning another
  bare-name compiler) are still intercepted: `.bear/` stays at the
  front of the child's PATH
- The user is not required to strip any directory from PATH, unset
  any environment variable, or configure `CCACHE_*` manually
- If every `gcc` on PATH is a masquerade wrapper and no real compiler
  can be found past them, Bear reports a diagnostic and skips
  registering that compiler (it does not fall back to the wrapper)

## Implementation details

### Detection

For each compiler that Bear resolves during wrapper setup (from
`CC`/`CXX`/... env vars or PATH discovery), Bear classifies the
resolved binary as a masquerade wrapper by:

1. Reading the file as a symbolic link (`read_link`, followed
   iteratively -- not `canonicalize`, which resolves too aggressively
   and would hide, for example, `/usr/bin/gcc -> gcc-13`).
2. Taking the final target's file name and comparing it, lowercased,
   against a fixed set of known wrapper names: `ccache`, `distcc`,
   `icecc`, `colorgcc`, `buildcache`.

If the match succeeds, the directory containing the resolved binary is
flagged as a masquerade directory. The resolution retries with that
directory removed from the search PATH. The process repeats until it
lands on a non-masquerade compiler or exhausts PATH.

If a non-masquerade compiler is not found, Bear logs a warning and
does not register a wrapper for that name. The build will see its
normal PATH, the same as if Bear were not involved; this is strictly
better than registering a wrapper that loops.

### Scope of the change

- `bear/src/intercept/environment.rs`:
  - `resolve_program_path` -- used for `CC=gcc`-style env vars
  - `compiler_candidates` -- used for PATH-based discovery when no
    compilers are configured
- Both paths share a helper that filters masquerade directories and
  reruns the search.
- The child process's PATH is not modified; only Bear's own lookup
  PATH is filtered. Masquerade directories remain visible to the
  build, which matters if, for example, a Makefile hard-codes
  `/usr/lib/ccache/gcc`; that call is unaffected and still intercepted
  only if Bear happens to have a wrapper for the basename.

### Interaction with existing code

- The manual workaround `ccache_free_path_and_compiler` in
  `integration-tests/tests/cases/intercept.rs` becomes unnecessary
  once this is in. Tests that use it are rewritten to rely on Bear
  itself stripping the masquerade dir, so that the test also protects
  this requirement against regression.

## Non-functional constraints

- Detection must be pure filesystem inspection. No subprocess may be
  spawned to identify a wrapper (cost, trust).
- Resolution failure for one compiler must not fail Bear overall;
  other compilers are still registered.
- The set of recognised wrapper names is fixed in source. Uncommon
  or locally built wrappers that do not match are not detected; the
  user can either unset them from PATH or use preload mode.

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
> then Bear logs a warning naming the compiler and the detected
> wrapper,
> and does not register a `.bear/` wrapper for it,
> and the build uses the compiler directly without Bear interception
> for that name.

Given a nested compiler invocation (a compiler-driver calls another
bare-name compiler from the child process):

> When the child invokes `cc -c foo.c`,
> then `.bear/cc` is still first on PATH in the grandchild process,
> so the invocation is intercepted.

### CI coverage

The existing `rust CI` workflow (`.github/workflows/build_rust.yml`)
runs integration tests on `ubuntu-latest`. The Ubuntu matrix entry
runs `apt-get install -y ccache` before `cargo test`, which creates
`/usr/lib/ccache/*` symlinks. The job does NOT prepend that dir to
PATH: putting ccache first on the job PATH would inflate event
counts for every preload-mode test that asserts an exact number of
compiler invocations.

At build-time, `integration-tests/build.rs` scans well-known
locations (`/usr/lib/ccache`, `/usr/lib64/ccache`,
`/usr/libexec/ccache`) for a ccache masquerade directory and, if
found, exposes it via the `CCACHE_MASQUERADE_DIR` env var and sets
`cfg(host_has_ccache_masquerade)`. The dedicated recursion test is
gated on that cfg. At runtime the test prepends
`CCACHE_MASQUERADE_DIR` to its own child PATH, exercising the
recursion scenario regardless of the host's default PATH while
leaving other tests ccache-free.

## Notes

### Alternatives considered and rejected

**Setting `CCACHE_COMPILER` in the wrapper's child environment.**
The original proposal. Rejected because the path the wrapper knows
IS the ccache symlink (that is what `which gcc` returned at setup),
and `CCACHE_COMPILER` pointing at a symlink-to-ccache makes ccache
recurse into itself. Empirically verified: on Fedora,
`CCACHE_COMPILER=/usr/lib64/ccache/gcc ccache gcc -c foo.c` hangs and
must be killed; `CCACHE_COMPILER=/usr/bin/gcc` works. The fix would
have required also resolving past ccache to get the real path --
which is precisely what this requirement does, making `CCACHE_COMPILER`
redundant. It is also ccache-specific and would not help with icecc,
distcc, or any other wrapper that lacks an equivalent variable.

**`CCACHE_PATH` alternative.** Set `CCACHE_PATH` to PATH minus
`.bear/`. Rejected: ccache-specific (no equivalent for other
wrappers), requires enumerating a safe PATH anyway, and does not
address the deeper issue (Bear's config pointing at the wrong
executable).

**Removing masquerade directories from the child's PATH.** Rejected:
masquerade directories might contain binaries other than the ones
that loop (e.g. some installs put `distcc` itself in the same dir);
stripping them globally would be heavy-handed. Filtering Bear's own
lookup PATH is the narrower intervention.

### Related

- Issue #445 -- original PATH-ordering report
- Issue #686 -- bare-name CC resolution (`wrapper_mode_resolves_cc_bare_name_via_path`)
- Related requirement: `interception-wrapper-mechanism`
- ccache 4.x manual: https://ccache.dev/manual/4.10.2.html
- icecream masquerade setup: https://github.com/icecc/icecream

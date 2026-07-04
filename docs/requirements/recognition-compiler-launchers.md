---
title: Recognize compiler-launcher invocations
status: in-progress
---

## Intent

When a build compiles through a compiler-launcher command (ccache,
distcc, sccache, and eventually icecc) that carries the real compiler in
its own arguments, Bear records the compilation as the real compiler's
invocation, parsed with that compiler's flag semantics -- not the
launcher's. Executables that exist only to masquerade as a compiler (a
launcher's own symlink farm, e.g. `/usr/lib64/ccache/gcc`) are never
treated as if they were the real compiler.

## Acceptance criteria

- An execution of ccache, distcc, or sccache with a real compiler named
  in its arguments (`ccache gcc -c main.c`, `distcc -j4 clang -c
  main.c`, `sccache clang++ -c main.c`) yields a database entry for the
  real compiler, parsed with that compiler's flag semantics.
- distcc's own flags (`-j`, `--jobs`, `-v`, `--verbose`, `-i`,
  `--show-hosts`, `--scan-avail`, `--show-principal`) are skipped when
  locating the real compiler in argv; they are never mistaken for the
  compiler name.
- A launcher invocation whose argument does not name a recognized
  compiler (for example `ccache make all`), or that has no inner
  argument at all (bare `ccache`), yields no entry.
- A launcher wrapping another launcher (`ccache distcc gcc -c main.c`)
  is not unwrapped; it yields no entry. Bear does not chase a chain of
  launchers.
- An executable that resolves (after canonicalization) to a launcher
  binary while being reached under a compiler's own name -- a launcher
  masquerade symlink farm -- is never probed as if it were a compiler;
  it is classified as the launcher and handled by the same unwrap path
  as an explicit launcher invocation.
- icecc, once recognized, follows the identical contract: the real
  compiler in its arguments is recorded as the compilation, and icecc's
  own flags are skipped the same way distcc's are.

## Non-functional constraints

- The set of recognized launcher names is fixed in source; a locally
  built or uncommon launcher that does not match a known basename is not
  recognized.

## Testing

Given `ccache gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc`,
> and the recorded arguments are `gcc -c main.c` (the `ccache` token is
> dropped; the real compiler's argv survives).

Given `distcc -j 4 gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc`,
> and the `-j 4` distcc-only flags do not appear in the recorded
> arguments.

Given `sccache clang -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `clang`.

Given `ccache make all` (the launcher's argument is not a compiler):

> When Bear recognizes it,
> then no database entry is produced.

Given `ccache distcc gcc -c main.c` (a launcher wrapping another
launcher):

> When Bear recognizes it,
> then no database entry is produced.

Given an executable reached through a launcher's masquerade symlink farm
under a compiler's own name (e.g. `/usr/lib64/ccache/gcc` resolving to
the `ccache` binary):

> When Bear recognizes it,
> then it is classified as the launcher, not probed as a compiler, and
> unwrapped the same way an explicit `ccache gcc ...` invocation would
> be.

Given icecc support is implemented (a later phase):

> When Bear recognizes `icecc gcc -c main.c`,
> then the recorded compiler is `gcc`, following the same contract as
> ccache/distcc/sccache above.

## Notes

- This requirement documents a contract that predates it: ccache,
  distcc, and sccache launcher-unwrap behavior has existed in Bear for
  several releases (`crates/bear/src/semantic/interpreters/compilers/wrapper.rs`).
  It is written down now because the icecc addition needs an existing
  contract to extend.
- icecc support itself is not yet implemented; this requirement stays
  `in-progress` until it lands. The Testing section already includes
  icecc's scenario so the contract does not need a second revision when
  the code arrives.
- Distinct from `interception-wrapper-recursion`, which covers a
  different mechanism: PATH-masquerade loop prevention in Bear's own
  wrapper-mode interception (the `bear-wrapper` binary standing in for
  the real compiler on macOS/Windows). This requirement covers
  recognition-time classification of an explicit launcher invocation
  appearing in the build's own argv, in any interception mode.

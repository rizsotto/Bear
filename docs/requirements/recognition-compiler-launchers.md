---
title: Recognize compiler-launcher invocations
status: implemented
---

## Intent

When a build compiles through a compiler-launcher command (ccache,
distcc, sccache, icecc) that carries the real compiler in
its own arguments, Bear records the compilation as the real compiler's
invocation, parsed with that compiler's flag semantics -- not the
launcher's.

## Acceptance criteria

- An execution of ccache, distcc, sccache, or icecc with a real
  compiler named in its arguments (`ccache gcc -c main.c`, `distcc -j4
  clang -c main.c`, `sccache clang++ -c main.c`, `icecc gcc -c
  main.c`) yields a database entry for the real compiler, parsed with
  that compiler's flag semantics.
- distcc's launcher-only options before the compiler are skipped when
  locating the real compiler in argv; they are never taken as the
  compiler name (the recognized set: `-j`, `--jobs`, `-v`,
  `--verbose`, `-i`, `--show-hosts`, `--scan-avail`,
  `--show-principal`).
- A launcher invocation whose argument does not name a recognized
  compiler (for example `ccache make all`), or that has no inner
  argument at all (bare `ccache`), yields no entry.
- A launcher wrapping another launcher (`ccache distcc gcc -c main.c`)
  is not unwrapped; it yields no entry. Bear does not chase a chain of
  launchers.
- A launcher invoked by its own basename (`ccache`, `distcc`,
  `sccache`, `icecc`) is never version-probed; the launcher contract
  above applies directly. An ambiguous compiler name that
  canonicalizes to a launcher binary (a masquerade link) is probed as
  invoked; that contract is owned by `recognition-ambiguous-name-probe`.
- A masquerade entry under a specific compiler's name (the launcher's
  symlink farm shadowing `gcc` on PATH) is recorded under that
  compiler's name with the invocation's own arguments; collapsing it
  with the re-executed real compiler in preload mode is owned by
  `output-duplicate-detection`.

## Non-functional constraints

- The set of recognized launcher names is fixed in source; a locally
  built or uncommon launcher that does not match a known basename is not
  recognized.

## Testing

Given `ccache gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc`,
> and the recorded arguments are `gcc -c main.c` (the `ccache` token is
> dropped; the real compiler's argv survives),
> and no version probe process is spawned for the `ccache` name --
> a launcher's own basename is not ambiguous. This no-probe observable
> holds for every launcher basename (`ccache`, `distcc`, `sccache`,
> `icecc`).

Given `distcc -j 4 gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc`,
> and the `-j 4` distcc-only tokens do not appear in the recorded
> arguments.
> This scenario is representative of every launcher-only option in
> the recognized set above.

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

Given a bare `ccache` execution with no arguments at all:

> When Bear recognizes it,
> then no database entry is produced.

Given a ccache masquerade symlink shadowing `gcc` on PATH
(`/usr/lib/ccache/gcc` resolving to the ccache binary), invoked as
`gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc` as observed,
> and the recorded arguments are the invocation's own
> (`gcc -c main.c`).
> Collapsing this entry with the re-executed real compiler in preload
> mode is owned by `output-duplicate-detection`.

Given `icecc gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc`, following the same contract as
> ccache/distcc/sccache above.

## Notes

- icecream's `icerun` is deliberately not a launcher: it runs arbitrary
  commands on the icecream cluster, not compiler invocations, so
  recognizing it would record non-compilations. Same reasoning as the
  `mpirun`/`mpiexec` exclusion in `recognition-compiler-names`.
- Distinct from `interception-wrapper-recursion`, which owns loop
  prevention for Bear's own wrapper-mode interception; this file owns
  recognition of launcher invocations appearing in the build's own argv.

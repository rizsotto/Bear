---
title: Recognize compiler-launcher invocations
status: implemented
---

## Intent

When a build compiles through a compiler-launcher command (ccache,
distcc, sccache, icecc) that carries the real compiler in
its own arguments, Bear records the compilation as the real compiler's
invocation, parsed with that compiler's flag semantics -- not the
launcher's. A launcher reached through its masquerade symlink farm is
never version-probed, so the launcher's own version banner can never
misclassify an ambiguous compiler name.

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
- An ambiguous compiler basename (one whose classification relies on
  the version probe) that resolves after canonicalization to a launcher
  binary is never probed: the launcher's version banner must not stand
  in for a compiler's. Such an execution yields no entry of its own; in
  preload mode the real compiler that the launcher re-executes is
  intercepted as a separate event and provides the entry.
- A masquerade entry under a specific compiler's name (the launcher's
  symlink farm shadowing `gcc` on PATH) is recorded under that
  compiler's name with the invocation's own arguments; the launcher
  re-executes the real compiler, and default duplicate detection
  collapses the pair in preload mode.
- icecc follows the identical contract: the real compiler in its
  arguments is recorded as the compilation. In its launcher usage icecc
  takes the compiler as its first argument with no launcher-specific
  flags before it, so nothing needs skipping (unlike distcc).

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

Given an ambiguous compiler name (`cc`) that canonicalizes to a
launcher binary (a masquerade symlink farm first on PATH):

> When Bear recognizes it,
> then the version probe is not run,
> and the launcher's version banner does not classify the name as any
> compiler.

Given `icecc gcc -c main.c`:

> When Bear recognizes it,
> then the recorded compiler is `gcc`, following the same contract as
> ccache/distcc/sccache above.

## Notes

- This requirement documents a contract that predates it: ccache,
  distcc, and sccache launcher-unwrap behavior has existed in Bear for
  several releases (`crates/bear/src/semantic/interpreters/compilers/wrapper.rs`).
  It was written down when the icecc addition needed an existing
  contract to extend; icecc support landed with this requirement's
  implementation.
- icecream's `icerun` is deliberately not a launcher: it runs arbitrary
  commands on the icecream cluster, not compiler invocations, so
  recognizing it would record non-compilations. Same reasoning as the
  `mpirun`/`mpiexec` exclusion in `recognition-compiler-names`.
- Distinct from `interception-wrapper-recursion`, which covers a
  different mechanism: PATH-masquerade loop prevention in Bear's own
  wrapper-mode interception (the `bear-wrapper` binary standing in for
  the real compiler on macOS/Windows). This requirement covers
  recognition-time classification of an explicit launcher invocation
  appearing in the build's own argv, in any interception mode.

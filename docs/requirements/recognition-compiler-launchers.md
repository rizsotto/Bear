---
title: Recognize compiler-launcher invocations
status: implemented
---

## Intent

Some builds compile through a compiler launcher, such as ccache or
distcc, that carries the real compiler in its own arguments. Bear
records such a compilation as the real compiler's invocation, parsed
with that compiler's flag semantics, not the launcher's.

## Acceptance criteria

- **Given** `ccache gcc -c main.c`, **when** Bear recognizes it,
  **then** the recorded compiler is `gcc`, and the recorded arguments
  are `gcc -c main.c`. The launcher token is dropped; the real
  compiler's arguments survive.
- **Given** `sccache clang -c main.c` or `icecc gcc -c main.c`, **when**
  Bear recognizes it, **then** the recorded compiler is the real
  compiler (`clang`, `gcc`), the same as for ccache.
- **Given** `distcc -j 4 gcc -c main.c`, **when** Bear recognizes it,
  **then** the recorded compiler is `gcc`, and the `-j 4` tokens do not
  appear in the recorded arguments. The distcc options that Bear's
  launcher definition knows are skipped this way before the compiler,
  and are never taken as the compiler name.
- **Given** `ccache make all`, where the argument is not a recognized
  compiler, **when** Bear recognizes it, **then** no entry is produced.
- **Given** a bare `ccache` with no arguments, **when** Bear recognizes
  it, **then** no entry is produced.
- **Given** `ccache distcc gcc -c main.c`, a launcher wrapping another
  launcher, **when** Bear recognizes it, **then** no entry is produced.
  Bear does not chase a chain of launchers.
- **Given** a launcher invoked by its own basename, **when** Bear
  recognizes it, **then** no version probe is spawned for that name.
  A launcher's own basename is not ambiguous. An ambiguous compiler
  name that resolves to a launcher binary (a masquerade link) is
  probed as invoked; that is owned by `recognition-ambiguous-name-probe`.
- **Given** a ccache masquerade symlink that shadows `gcc` on PATH
  (`/usr/lib/ccache/gcc` resolving to the ccache binary), invoked as
  `gcc -c main.c`, **when** Bear recognizes it, **then** the recorded
  compiler is `gcc` as observed, and the recorded arguments are the
  invocation's own (`gcc -c main.c`). Collapsing this entry with the
  real compiler it runs under the preload mechanism is owned by
  `output-duplicate-detection`.
- **Given** `icerun gcc -c main.c` (icecream's `icerun`), **when** Bear
  recognizes it, **then** that execution yields no entry. `icerun` is
  not a launcher.
- This file owns launchers that appear in the build's own commands.
  Loop prevention for Bear's own wrappers under the wrapper mechanism is
  owned by `interception-wrapper-recursion`.

## Known limitations

**Fixed set of launcher names**: Bear recognizes a launcher by its
basename against a fixed set. A locally built or uncommon launcher with
another name is not recognized. Configuring a launcher at another
basename does not make it recognized either.

## Rationale

- [Some compiler-adjacent names are deliberately not recognized](../rationale/names-deliberately-not-recognized.md)
- [A compiler family is defined by its YAML file](../rationale/compiler-family-definition.md)

---
title: Disambiguate ambiguous compiler names by version probe
status: implemented
---

## Intent

Some compiler names do not tell which toolchain is behind them. `cc` and
`c++` are GCC on most Linux hosts and Clang on the BSDs and macOS. On HPE
Cray systems, the PrgEnv wrapper `CC` drives whichever compiler the
loaded programming environment selects. When a build invokes such a
name, Bear must parse the call with the right compiler's flag semantics.
A wrong guess corrupts the entry: Clang's `-Xclang <arg>` takes the next
argument, GCC's flag table does not, so the source and output are
misread.

The user expects `bear -- cc -c hello.c` to give a correct entry on
every host, with no per-platform configuration, and with a way to
override Bear's guess.

## Acceptance criteria

- **Given** a host where `/usr/bin/cc` is Clang (or GCC), **when** Bear
  recognizes `cc -c hello.c`, **then** it runs `cc --version` once and
  parses the call with the family the version output names. On the Clang
  host, `-Xclang -ast-dump` is parsed with the right arity. `c++`
  behaves the same way.
- **Given** an HPE Cray host where `CC --version` prints the loaded
  programming environment's banner, **when** Bear recognizes
  `CC -c hello.cpp`, **then** it parses the call with the family that
  banner names: Clang under PrgEnv-cray (the CCE Clang banner), GCC under
  PrgEnv-gnu (the FSF GCC banner).
- **Given** a name outside the ambiguous set (`gcc -c hello.c`), **when**
  Bear recognizes it, **then** no probe runs. Static name recognition,
  owned by `recognition-compiler-names`, classifies it.
- **Given** a cross-prefixed `cc` or `c++` (`aarch64-linux-gnu-cc`),
  **when** Bear recognizes it, **then** it is neither probed nor
  recognized, and yields no entry. The user sets its family in the
  configuration.
- **Given** one `cc` path invoked 1000 times in one run, **when** Bear
  recognizes them, **then** the executable is run at most once for
  probing. Paths that resolve to the same file share one probe.
- **Given** an executable that hangs on `--version`, **when** Bear
  probes it, **then** the probe returns within a fixed, built-in
  timeout (2 seconds), and the execution yields no entry.
- **Given** an executable that reads stdin on `--version` (a stray
  `bash` named `cc` on PATH), **when** Bear probes it, **then** the
  probe does not block, because stdin is closed.
- **Given** a `cc` whose `--version` output has no known signature (a
  custom wrapper with a vendor banner), **when** Bear recognizes it,
  **then** the probe declines, the execution is not classified, and it
  yields no entry. Bear never falls back to a guess by name. The same
  holds when the probe exits non-zero, cannot be started, or is not
  available on the platform.
- **Given** Bear running with its preload library in the environment,
  **when** it probes a compiler, **then** the probe runs without the
  preload library and is not recorded as a build event.
- **Given** a configuration that maps `/usr/bin/cc` to GCC, **when**
  Bear recognizes `/usr/bin/cc`, or a bare `cc` that the entry matches,
  **then** the result is GCC, no probe runs, and the recorded compiler
  stays as observed. A configured family is the only way to switch the
  probe off, per path. How an entry matches an execution is owned by
  `recognition-compiler-names`.
- **Given** two configured entries whose paths share the filename `cc`
  but map it to different families, **when** Bear recognizes a bare
  `cc`, **then** the first entry classifies it and a warning names the
  conflict. Each full path keeps its own entry's family.
- **Given** a bare ambiguous name, **when** Bear probes it, **then** it
  runs the name as spelled, found through Bear's own PATH. The recorded
  compiler is not rewritten (owned by
  `output-json-compilation-database`).
- **Given** `/usr/lib/ccache/cc`, a masquerade link that resolves to the
  ccache binary, **when** Bear recognizes it, **then** it probes the path
  as invoked, the masquerade passes `--version` through to the real
  compiler, the call is parsed with the family that output names, and
  the recorded compiler stays `/usr/lib/ccache/cc`. Each masquerade name
  is probed on its own, not shared with others that resolve to the same
  launcher.
- **Given** the same masquerade link whose passthrough output has no
  known signature, **when** Bear recognizes it, **then** the probe
  declines and the execution yields no entry.
- A compiler launcher invoked by its own name is never probed. That
  contract is owned by `recognition-compiler-launchers`.

## Known limitations

The probe knows only the Clang and GCC version banners. A compiler with
another banner, for example `nvc++` under PrgEnv-nvidia, stays
unrecognized.

## Rationale

- [Cached version probe as sole classifier](../rationale/ambiguous-cc-version-probe.md)

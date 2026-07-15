---
title: Disambiguate ambiguous compiler names by version probe
status: implemented
---

## Intent

When a build invokes a compiler under an ambiguous name (notably `cc`,
`c++`, and the HPE Cray PrgEnv wrapper `CC`), Bear must dispatch to the
correct interpreter (GCC vs Clang) regardless of which toolchain the
system actually installs under that name. On Linux `cc` is typically
GCC; on FreeBSD, OpenBSD, NetBSD, DragonFly, and macOS `cc` is Clang.
On HPE Cray systems, `CC` drives whichever compiler module the loaded
programming environment selects. Misidentifying the compiler causes
flag-arity mistakes (e.g., Clang's `-Xclang <arg>` consumes the next
argv slot, GCC's flag table does not), which corrupts source/output
detection in the compilation database.

The user expects `bear -- cc -c hello.c` to produce a correct entry on
every host, without per-platform configuration, and without losing the
ability to override Bear's guess when needed.

## Acceptance criteria

- For executables whose basename matches a known-ambiguous name (`cc`,
  `c++`, `CC`), Bear runs the executable once with `--version` to
  classify it as GCC or Clang before dispatching to an interpreter.
- The probe runs at most once per distinct probe key for the lifetime of
  the process; subsequent invocations of the same path reuse the cached
  result. The key is the canonical executable path -- except for a
  masquerade link (below), whose key is the invoked path, because the
  answer depends on the name the wrapper was called by.
- The probe is the sole classifier for ambiguous names. Static name
  recognition (owned by `recognition-compiler-names`) deliberately
  excludes the ambiguous names, so when the probe declines (timeout,
  unrecognizable output, failed spawn, non-zero exit) or cannot run at
  all (a platform where the probe is unavailable), the execution is
  not classified and no database entry is produced for it -- Bear never
  guesses.
- Probes never deadlock. Stdin is closed; the call returns within a bounded
  time budget even for hung children.
- Probes do not recursively re-enter Bear's own interception.
- A user compiler-override configuration entry mapping a path to a
  compiler family (the man page names the key) takes priority over the
  probe and disables it for that path. The entry matches the executable
  as the build spelled it, after canonicalization, or -- when the build
  spelled a bare name -- by the configured path's filename. When two
  entries share a filename but disagree on the compiler, the first entry
  classifies bare invocations and a warning names the conflict. This is
  the supported override mechanism and the only way to recover
  recognition for a quirky `cc` whose `--version` output does not match
  the probe's signature rules.
- A path invoked by a wrapper's own basename (`ccache`, `distcc`,
  `sccache`) is never probed: those names are not ambiguous and the
  wrapper interpreter handles them. An ambiguous-named path whose
  canonical target is such a wrapper -- a masquerade link like
  `/usr/lib/ccache/cc` -- IS probed, as invoked: a masquerade binary
  answers `--version` in the name it was called by, passing the flag
  through to the underlying compiler, so the probe classifies the real
  toolchain. Unrecognizable passthrough output declines as usual.
- Names that are not in the ambiguous set are never probed; they
  resolve by the static name rules owned by `recognition-compiler-names`.

## Non-functional constraints

- The probe runs at most once per unique probe key (canonical path, or
  the invoked path for a masquerade link), not once per invocation.
- Probe timeout: short (single-digit seconds), bounded.
- The classification rule is conservative: when in doubt, return no
  classification rather than guess wrong.

## Known limitations

The probe classifies only the version banners it already recognizes
(Clang's and GCC's). A programming environment whose compiler prints a
banner the probe does not know -- for example `nvc++` under
PrgEnv-nvidia -- stays unrecognized. Extending the classifier to more
banners is out of scope.

## Testing

Given a host where `/usr/bin/cc` is Clang (or, symmetrically, GCC):

> When Bear recognizes an execution of `cc -c hello.c`,
> then it dispatches to the interpreter the probe identified
> (Clang or GCC respectively),
> and on the Clang host a Clang-only flag with a follow-on argument
> (e.g. `-Xclang -ast-dump`) is parsed with correct arity.

The `cc` scenarios in this section are representative of `c++`; both
names follow the same probe path.

Given an HPE Cray host where the PrgEnv wrapper `CC` answers
`--version` with the loaded programming environment's banner (the CCE
Clang banner under PrgEnv-cray, the FSF GCC banner under PrgEnv-gnu):

> When Bear recognizes an execution of `CC -c hello.cpp`,
> then it dispatches to the interpreter the banner names
> (Clang under PrgEnv-cray, GCC under PrgEnv-gnu).

Given an execution of `gcc -c hello.c` (a name outside the ambiguous
set):

> When Bear recognizes it,
> then static name recognition classifies it as GCC
> and no probe process is spawned.

Given a user config with two compiler-override entries whose paths
share the filename `cc` but map it to different compiler families
(the first to Clang, the second to GCC):

> When Bear recognizes an execution spelled as a bare `cc`,
> then the first entry classifies it (Clang),
> and a warning names the conflicting entries.
> An execution spelled with either full path keeps that entry's own
> classification.

Given any host:

> When Bear recognizes the same `cc` path 1000 times in one run,
> then the executable is spawned at most once for probing.

Given a user config containing `compilers: [{ path: /usr/bin/cc, as: gcc }]`:

> When Bear recognizes `/usr/bin/cc`,
> then the result is GCC and no probe is performed.

Given the same config:

> When Bear recognizes an execution spelled as a bare `cc`,
> then the result is GCC and no probe is performed,
> and the recorded compiler stays `cc` as observed.

Given an executable that hangs on `--version`:

> When Bear probes it,
> then the call returns within the configured timeout
> and no database entry is produced for the execution
> (there is no name-based fallback for ambiguous names).

Given an executable named `cc` whose `--version` output contains no
recognizable signature (e.g. a custom wrapper that prints a vendor
banner):

> When Bear recognizes it,
> then the probe declines, the execution is not classified,
> and no database entry is produced for it.
> The user can recover the entry with a compiler-override
> configuration entry that maps the path to a compiler family.

Given an executable that reads from stdin on `--version`
(e.g. a misplaced `bash` in PATH named `cc`):

> When Bear probes it,
> then the call does not block indefinitely
> (stdin is closed, so the read returns EOF and the process exits).

Given Bear is running with `LD_PRELOAD` set to its own interception library:

> When Bear probes a compiler,
> then the probe's environment has `LD_PRELOAD` removed
> and the probe execution is not itself recorded as a build event.

Given an executable named `/usr/lib/ccache/cc`
that resolves (after canonicalization) to the ccache wrapper:

> When Bear recognizes `/usr/lib/ccache/cc`,
> then it probes the path as invoked
> (`--version` passes through the masquerade to the underlying compiler),
> the classification follows the passthrough banner,
> and the recorded compiler stays `/usr/lib/ccache/cc` as observed.

Given the same masquerade link, where the passthrough `--version` output
matches no known signature:

> When Bear recognizes it,
> then the probe declines and no database entry is produced for
> the execution -- the same conservative failure as any other
> unclassifiable ambiguous name.

## Notes

- Override mechanism: by user request, the only way to disable the probe
  for a given path is a compiler-override configuration entry for that
  path. There is no process-wide off switch; the override is per-path
  and explicit.
- A bare ambiguous name is probed by executing it as spelled, letting
  the operating system resolve it through Bear's own `PATH`. Bear does
  not resolve the name itself, and it never rewrites the executable it
  records (see `output-json-compilation-database`).
- `CC` joined the ambiguous set for the HPE Cray PrgEnv wrapper (see
  the Recognized names table in `recognition-compiler-names` for the
  Cray CCE names and the statically recognized PrgEnv `ftn`
  wrapper): the same reasoning as `cc`/`c++` applies verbatim, no change
  to the classifier itself was needed. `CC` cannot be a statically
  recognized name either: a static mapping would be wrong on every
  programming environment except the one it hardcoded, and on a
  case-insensitive filesystem it would shadow `cc`.

## Rationale

- [Cached version probe as sole classifier](../rationale/ambiguous-cc-version-probe.md) -
  why the probe replaces per-OS defaults, a name-recognition fallback,
  and the per-invocation probe (PR #695).

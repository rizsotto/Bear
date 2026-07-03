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
- The probe runs at most once per distinct canonical executable path for
  the lifetime of the process. Subsequent invocations of the same path
  reuse the cached result.
- The probe is the sole classifier for ambiguous names. The compiler
  recognition regex (`gcc.yaml` and friends) deliberately does not list
  `cc`/`c++`, so when the probe declines (timeout, unrecognizable
  output, failed spawn, non-zero exit) recognition returns
  `NotRecognized` rather than guessing. A missing entry is visible and
  debuggable; a wrongly-classified entry corrupts the compilation
  database silently via mismatched flag-arity tables, which is the bug
  this requirement exists to prevent.
- Probes never deadlock. Stdin is closed; the call returns within a bounded
  time budget even for hung children.
- Probes do not recursively re-enter Bear's own interception.
- A user `compilers:` config entry for a path takes priority over the
  probe and disables it for that path. This is the supported override
  mechanism and the only way to recover recognition for a quirky `cc`
  whose `--version` output does not match the probe's signature rules.
- Wrapper basenames (`ccache`, `distcc`, `sccache`) are never probed even
  if they appear under an ambiguous name. The wrapper interpreter handles
  them as today.
- Names that are not in the ambiguous set (e.g. `gcc`, `clang`, `gfortran`,
  cross-prefixed or versioned variants) are not probed and continue to
  resolve via regex.
- On non-Unix targets (Windows) the probe is not available and the
  recognizer wires up a no-op probe. Windows toolchains use unambiguous
  basenames (`cl.exe`, `clang-cl`, `gcc.exe`) that the regex layer
  classifies directly. Bare `cc`/`c++` on Windows falls through to
  `NotRecognized`; in practice no Windows toolchain installs them.

## Non-functional constraints

- The probe runs at most once per unique canonical path, not once per
  invocation.
- Probe timeout: short (single-digit seconds), bounded.
- The classification rule is conservative: when in doubt, return no
  classification rather than guess wrong. A misclassification corrupts the
  database; a non-classification falls back to existing behavior.

## Testing

Given a host where `/usr/bin/cc` is Clang:

> When Bear recognizes an execution of `cc -c hello.c`,
> then it dispatches to the Clang interpreter,
> and a Clang-only flag with a follow-on argument
> (e.g. `-Xclang -ast-dump`) is parsed with correct arity.

Given a host where `/usr/bin/cc` is GCC:

> When Bear recognizes an execution of `cc -c hello.c`,
> then it dispatches to the GCC interpreter.

Given any host:

> When Bear recognizes the same `cc` path 1000 times in one run,
> then the executable is fork-exec'd at most once for probing.

Given a user config containing `compilers: [{ path: /usr/bin/cc, as: gcc }]`:

> When Bear recognizes `/usr/bin/cc`,
> then the result is GCC and no probe is performed.

Given an executable that hangs on `--version`:

> When Bear probes it,
> then the call returns within the configured timeout
> and the execution is reported as `NotRecognized`
> (no entry is written; there is no regex fallback for ambiguous names).

Given an executable named `cc` whose `--version` output contains no
recognizable signature (e.g. a custom wrapper that prints a vendor
banner):

> When Bear recognizes it,
> then the probe declines, recognition returns `None`,
> and the execution is reported as `NotRecognized`.
> The user can recover the entry by adding the path to
> `compilers:` with an explicit `as:` field.

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
> then it does not probe the binary
> and the wrapper interpreter handles the invocation as today.

## Notes

- Override mechanism: by user request, the only way to disable the probe
  for a given path is to declare it in `compilers:`. There is no
  process-wide off switch; the override is per-path and explicit.
- `gcc.yaml` carries a comment explaining why `cc`/`c++` are absent from
  its recognize list.
- `CC` joined the ambiguous set for the HPE Cray PrgEnv wrapper (see
  `recognition-cray-compilers.md`): the same reasoning as `cc`/`c++`
  applies verbatim, no change to the classifier itself was needed.

## Rationale

- [Cached version probe as sole classifier](../rationale/ambiguous-cc-version-probe.md) -
  why the probe replaces per-OS defaults, a regex fallback, and the
  per-invocation probe (PR #695).

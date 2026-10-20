# Disambiguating cc/c++: a cached version probe as sole classifier

## Context

`cc` and `c++` name different toolchains per OS - GCC on most Linux,
Clang on the BSDs and macOS. Misclassifying them corrupts the database
silently, because GCC and Clang have different flag-arity tables (e.g.
Clang's `-Xclang <arg>` consumes the next argv slot and GCC's does not),
so source/output detection mis-parses.

A masquerade link (`/usr/lib/ccache/cc` -> `ccache`) adds a twist. It
canonicalizes to the wrapper, but the recognition regex sees only the
invoked basename (`cc`), and the launcher interpreter cannot unwrap it,
because argv has no inner compiler token. If the probe does not classify
it, the event is dropped. A masquerade binary answers `--version` in the
name it was called by and passes the flag through to the underlying
compiler. Its own banner appears only when it is invoked as itself,
which is never an ambiguous name.

Options considered:

- **Per-OS defaults**: hard-code "cc means Clang on BSD/macOS, GCC
  elsewhere". Rejected: wrong whenever a host installs the other
  toolchain under `cc`, which is exactly the case users hit.
- **Regex with a probe fallback (probe-then-regex)**: on probe failure,
  fall back to the recognition regex, which defaults `cc` to GCC.
  Rejected: it re-introduced the very bug on the platforms the probe was
  meant to fix.
- **Per-invocation probe** ([PR #695](https://github.com/rizsotto/Bear/pull/695)):
  run `--version` on every invocation. Rejected: it costs a `fork+exec`
  per compiler call, where a cache costs one or two per build.
- **Do not probe masquerade links**, for fear of the wrapper's own
  banner. Rejected: the fear is unfounded (see above), and the events
  would be silently dropped.
- **Cached version probe as the sole classifier**: run the executable
  once with `--version`, cache the result per canonical path, and do not
  fall back to the regex for ambiguous names. Chosen.

## Decision

Use the cached version probe as the sole classifier for ambiguous names.
The recognition regex deliberately does not list `cc`/`c++`, so when the
probe declines (timeout, unrecognizable output, failed spawn, non-zero
exit) recognition returns `NotRecognized` rather than guessing. A
masquerade link is probed as invoked and cached under the invoked path,
because its answer depends on the name it was called by.

## Consequences

- The failure mode is **loud, not wrong**: a missing entry is visible and
  debuggable; a mis-classified entry corrupts the database silently.
  The GCC compiler definition carries a comment recording why `cc`/`c++`
  are absent from its recognize list.
- The user override is per-path and explicit: declaring a path under
  `compilers:` takes priority over the probe and is the only way to
  recover recognition for a quirky `cc` whose `--version` output the
  probe cannot read. There is no process-wide off switch, by request.
- Caching per canonical path keeps the cost at one or two `fork+exec`
  pairs per clean build rather than per invocation. Masquerade links are the
  one exception: the canonical key would collapse different masquerade
  names (`cc`, `c++`) onto the wrapper binary whose behavior depends on
  argv[0], so those cache under the invoked path.
- Probing through a masquerade link keeps the loud-not-wrong property:
  unrecognizable passthrough output (an exotic wrapper printing its own
  banner) still declines.
- The ambiguous-names list is intentionally minimal (`cc`, `c++`, `CC`).
  Cross-prefixed variants like `aarch64-linux-gnu-cc` are not probed and
  not recognized: the probe matches the exact name, and the GCC
  definition does not list `cc`. The user sets their family in the configuration. The
  list can grow if a real cross-toolchain case appears.
- The HPE Cray PrgEnv wrapper `CC` is probed for the same reason as
  `cc`. A static mapping would be wrong in every programming environment
  but the one it picked, and on a case-insensitive filesystem it would
  shadow `cc`.

## References

- [recognition-ambiguous-name-probe](../requirements/recognition-ambiguous-name-probe.md), the requirement this supports
- [PR #695](https://github.com/rizsotto/Bear/pull/695), the per-invocation probe variant, not taken

---
title: LD_PRELOAD-based command interception
status: implemented
---

## Intent

When the user runs `bear -- make` on Linux, Bear intercepts every process
execution that happens during the build by injecting a shared library into
the build process. The user does not need to modify the build system or
install special compiler wrappers -- Bear works transparently with any
build tool that spawns compiler processes.

On macOS the same mechanism uses `DYLD_INSERT_LIBRARIES` instead of
`LD_PRELOAD`. On both platforms the effect is the same: Bear sees every
`exec` call and reports it to the collector for semantic analysis.

## Acceptance criteria

- `exec` family calls, `posix_spawn`, `popen`, and `system` are
  intercepted
- Child processes inherit the interception environment even when the
  build system clears or replaces the environment
- Intercepted commands are reported back to Bear for analysis
- The build process completes normally -- interception does not alter
  build output, exit codes, or observable behavior
- Co-resident preload libraries (e.g. Gentoo's `libsandbox.so`) are
  preserved in the preload variable
- Reporting failures do not affect the build process
- The preload library path and collector address are communicated via
  environment variables, not hard-coded

## Non-functional constraints

- Must not alter build output or exit codes
- Must handle concurrent builds (parallel make) without losing reports
- Platform: Linux and BSD systems (`LD_PRELOAD`), macOS
  (`DYLD_INSERT_LIBRARIES`)
- Not supported on Windows (no equivalent mechanism)
- Not supported on macOS when SIP is enabled (the dynamic linker
  strips `DYLD_INSERT_LIBRARIES` for protected executables)
- Statically linked executables are not affected by the preload
  mechanism -- this is a fundamental limitation of the approach

## Known limitations

**Wrong ELF class during cross-compilation** (issue #236): the preload
library is built for the host architecture, so when the build invokes a
cross-compiler targeting a different architecture the dynamic linker
rejects the library with "wrong ELF class". The build still completes,
but the cross-compiled commands are not intercepted.

**glibc symbol-version mismatch in cross-compilation** (discussion
#707): the preload library is linked against the host's glibc. If it is
loaded into a compiler running against an older-glibc sysroot and
references a newer glibc symbol version, that invocation fails with a
"version not found" error and the command is not recorded.

**macOS SIP** (issue #558): System Integrity Protection strips
`DYLD_INSERT_LIBRARIES` for system executables. Bear detects SIP at
startup and falls back to wrapper mode; users who disable SIP can force
preload mode via configuration.

**Preload conflicts with sandboxes** (issue #699): a co-resident
`LD_PRELOAD` sandbox library that hooks the same `exec` family can
re-assert its own preload variable downstream in the exec chain and drop
Bear's entry, so a grandchild past that point is not intercepted. Bear
cannot prevent this without refusing to delegate to the other library,
which would disable the sandbox and alter the build.

**Affects all child processes** (issue #444): `LD_PRELOAD` applies to
every process spawned during the build, not just compilers, which can
disturb non-compiler tools sensitive to preloaded libraries. The
semantic analysis layer filters non-compiler commands from the output,
but the preload injection itself cannot be selective.

## Testing

Given a project with a single C source file on Linux:

> When the user runs `bear -- cc -c test.c`,
> then `compile_commands.json` is created with one entry for `test.c`,
> and the build exit code is preserved (zero for success).

Given a build system that clears the environment:

> When a build script runs `env -i cc -c test.c` and the compiler is
> launched via `execve` (or another function with an explicit `envp`),
> then the preload library restores `LD_PRELOAD` in the child,
> and the compilation is still intercepted and appears in the output.
> Note: `execvp` does not receive explicit environment doctoring; if
> the build uses `execvp` after stripping `LD_PRELOAD`, grandchild
> processes may not be intercepted.

Given a parallel build with multiple source files:

> When the user runs `bear -- make -j4` on a project with four source
> files,
> then all four compilations appear in `compile_commands.json`,
> and no reports are lost due to concurrent TCP connections.

Given a build whose last compiler reports immediately before the build
process exits:

> When that final report races the shutdown of interception,
> then it is still delivered before interception stops,
> and that last compilation still appears in `compile_commands.json`
> (no entry is lost to the shutdown race -- see issue #704).

Given a build that invokes non-compiler commands:

> When the build runs `cp`, `mkdir`, and `cc -c test.c`,
> then all three executions are reported to the collector,
> but only the `cc` invocation appears in the final compilation database
> (non-compiler commands are filtered by semantic analysis, not by the
> preload library).

Given an existing `LD_PRELOAD` value in the environment:

> When the user has `LD_PRELOAD=/usr/lib/libsandbox.so` set before
> running Bear,
> then the effective `LD_PRELOAD` contains Bear's library first,
> followed by `/usr/lib/libsandbox.so`,
> and both libraries are preserved in child processes.

Given a build on macOS with SIP disabled:

> When the user forces preload mode via configuration,
> then `DYLD_INSERT_LIBRARIES` and `DYLD_FORCE_FLAT_NAMESPACE=1` are
> set,
> and compiler invocations are intercepted the same way as on Linux.

Given a build on macOS with SIP enabled:

> When Bear detects SIP is active,
> then preload mode is not available,
> and Bear uses wrapper mode instead (see `interception-wrapper-mechanism`).

## Notes

- The preload library path is resolved at runtime, so a Bear installed
  via a package manager loads the library from its install location
  rather than a build-time path (issues #648, #649, #597, #582).
- Internal compiler invocations (`cc1`, `cc1plus`, `collect2`, etc.)
  are intercepted and reported but filtered out during semantic analysis,
  not in the preload library itself. See `output-json-compilation-database`
  for details on which commands appear in the output.
- Related requirement: `interception-wrapper-mechanism` (alternative
  interception mode).

## Rationale

- [Linker selection for the preload library](../rationale/preload-linker-selection.md)

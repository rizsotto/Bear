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
- Intercepted commands are reported to the TCP collector
- The build process completes normally -- interception does not alter
  build output, exit codes, or observable behavior
- Co-resident preload libraries (e.g. Gentoo's `libsandbox.so`) are
  preserved in the preload variable
- Reporting failures do not affect the build process
- The preload library path and collector address are communicated via
  environment variables, not hard-coded

## Non-functional constraints

- Must not alter build output or exit codes
- Must handle concurrent builds (parallel make) -- each intercepted
  execution opens its own TCP connection
- Platform: Linux and BSD systems (`LD_PRELOAD`), macOS
  (`DYLD_INSERT_LIBRARIES`)
- Not supported on Windows (no equivalent mechanism)
- Not supported on macOS when SIP is enabled (the dynamic linker
  strips `DYLD_INSERT_LIBRARIES` for protected executables)
- Statically linked executables are not affected by the preload
  mechanism -- this is a fundamental limitation of the approach

## Known limitations

**Wrong ELF class during cross-compilation** (issues #236, #510, #517,
#555): The preload library is compiled for the host architecture. When
the build invokes cross-compilers targeting a different architecture,
the dynamic linker rejects the library with "wrong ELF class". This
produces warning messages but does not prevent the build from
completing. The cross-compiled commands are not intercepted.

**glibc symbol-version mismatch in cross-compilation** (discussion
#707): The preload library is linked against the host's glibc. The
dynamic linker loads it into every intercepted process, including
compilers that run against a cross-compilation SDK sysroot with an
older glibc. If the library references a glibc symbol version newer
than the sysroot's libc provides (e.g. `GLIBC_2.33`), the intercepted
invocation fails with a "version not found" error and that command is
not recorded. Unlike the wrong-ELF-class case (a different
architecture), here the architecture matches and only the glibc
version differs. Workaround: build Bear on a host whose glibc is no
newer than the SDK's libc, or distribute a Bear built against a
compatible glibc alongside the SDK.

**macOS SIP** (issues #108, #152, #232, #360, #558): System Integrity
Protection strips `DYLD_INSERT_LIBRARIES` for system executables. Bear
detects SIP at startup via `csrutil status` and falls back to wrapper
mode. Users who disable SIP can force preload mode via configuration.

**Preload conflicts with sandboxes** (issues #675, #699): Gentoo's
sandbox (`libsandbox.so`) is itself an `LD_PRELOAD` library hooking the
same `exec` family. When a build step clears the environment (`env -i`)
and re-execs, Bear re-inserts its library first, but a co-resident
sandbox library downstream in the exec chain can re-assert its own
`LD_PRELOAD` and drop Bear's entry, so the grandchild is not
intercepted. Bear cannot prevent this without refusing to delegate to
the other library, which would disable the sandbox and alter the build.
This surfaces when Bear's own test suite is run *inside* the sandbox
(e.g. `FEATURES=test` during `emerge`); the fix is packaging-side -
keep `RESTRICT="test"` or run the test phase with the sandbox disabled
(`FEATURES="-sandbox -usersandbox"`). Non-sandboxed interception is
unaffected. See bugs.gentoo.org/973619.

**Affects all child processes** (issues #444, #556): `LD_PRELOAD`
applies to every process spawned during the build, not just compilers.
This can cause failures in non-compiler tools that are sensitive to
preloaded libraries (e.g. tools with incompatible `libstdc++`
dependencies). The semantic analysis layer filters non-compiler commands
from the output, but the preload injection itself cannot be selective.

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

> When that final report is still queued in the collector's accept
> backlog at the moment shutdown is requested,
> then the collector drains the backlog before stopping,
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

- The preload library path must be correct at runtime. When Bear is
  installed via a package manager, the default config must point to the
  installed library location, not the build-time path. Issues #648,
  #649, #597, #582 were caused by stale build-time paths in the default
  configuration.
- Internal compiler invocations (`cc1`, `cc1plus`, `collect2`, etc.)
  are intercepted and reported but filtered out during semantic analysis,
  not in the preload library itself. See `output-json-compilation-database`
  for details on which commands appear in the output.
- Related requirement: `interception-wrapper-mechanism` (alternative
  interception mode).

## Rationale

- [Linker selection for the preload library](../rationale/preload-linker-selection.md)

---
title: LD_PRELOAD-based command interception
status: implemented
tests:
  - basic_command_interception
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

## Implementation details

### Activation

Preload mode is the **default** on Linux and BSD systems (FreeBSD,
NetBSD, OpenBSD, DragonFly BSD). On macOS it is available only when
System Integrity Protection (SIP) is disabled; when SIP is enabled,
Bear falls back to wrapper mode (see `interception-wrapper-mechanism`).
Bear detects SIP at startup via `csrutil status`; if `csrutil` is
absent or fails, SIP is assumed disabled and preload mode proceeds.

The user can force preload mode via the configuration file:

```yaml
schema: "4.1"
intercept:
  mode: preload
```

There is no CLI flag for mode selection -- it is configuration-only.

### Injection

When Bear launches the build command, it sets environment variables that
cause the dynamic linker to load Bear's shared library into every child
process:

- **Linux and BSD systems**: `LD_PRELOAD=<path-to-libexec.so>`
- **macOS**: `DYLD_INSERT_LIBRARIES=<path-to-libexec.so>` and
  `DYLD_FORCE_FLAT_NAMESPACE=1`

The library is inserted **first** in the preload variable so that it
takes precedence. Any existing preload entries (e.g. Gentoo's
`libsandbox.so`) are preserved after Bear's library.

A second variable, `BEAR_INTERCEPT`, carries serialized session state
(the TCP collector address and library path) so the library knows where
to report.

### Interception

The shared library overrides libc functions that execute programs. When
any of these functions is called, the library:

1. Reports the execution (executable path, arguments, working directory)
   to the TCP collector.
2. Restores (or "doctors") the preload environment in the child process
   so that grandchild processes are also intercepted.
3. Calls the real libc function via `dlsym(RTLD_NEXT, ...)`.

Intercepted functions:

| Family | Functions |
|---|---|
| exec | `execl`, `execlp`, `execle`, `execv`, `execve`, `execvp`, `execvpe`, `execvP`, `exect` |
| posix_spawn | `posix_spawn`, `posix_spawnp` |
| shell | `popen`, `system` |

Not all functions exist on all platforms. The library checks at compile
time which symbols are available (`has_symbol_*` checks in
`platform-checks`) and only wraps those that exist.

### Environment doctoring

Some build systems clear or replace the process environment (e.g.
`env -i make`). If the preload library detects that `LD_PRELOAD` or
`BEAR_INTERCEPT` has been removed from the child environment, it
restores them before calling the real exec function. This ensures
interception survives environment resets.

The library captures a snapshot of the preload variable at startup
(process load time). When restoring, it uses this snapshot as the base
so that co-resident preload libraries are also preserved.

Functions that accept an explicit `envp` parameter (`execve`,
`execvpe`, `posix_spawn`, `posix_spawnp`, `exect`) have their
environment doctored via the parameter. `popen` and `system` are
reimplemented internally using `posix_spawnp` with a doctored `envp`,
so they receive the same protection. `execvp` (and `execlp` on
platforms where `execvpe` is unavailable) does not receive explicit
environment doctoring; it relies on the process `environ`. If the
build system strips `LD_PRELOAD` from `environ` before calling
`execvp`, grandchild processes may not be intercepted.

### Reporting

Each intercepted execution is reported over a fresh TCP connection to
a collector running on the loopback interface. The wire format is
Length-Value: a 4-byte big-endian length prefix followed by a JSON
payload. The collector listens on a dynamically assigned port to avoid
conflicts with other Bear instances or parallel builds.

If reporting fails (e.g. the collector has already shut down), the
library silently ignores the error. The build process is never affected
by reporting failures.

### Split C/Rust implementation

The shared library uses a two-layer architecture:

- **C shim** (`intercept-preload/src/c/shim.c`): Thin wrappers that
  handle variadic arguments (`execl`, `execlp`, `execle`) which stable
  Rust cannot express. Uses a two-pass approach (count args, then
  extract into a stack VLA). All exported ELF/dylib symbols are defined
  in C rather than Rust to avoid recursive interception on FreeBSD.
- **Rust core** (`intercept-preload/src/implementation.rs`): The actual
  interception logic -- reporting, environment doctoring, and calling
  the real function via dlsym.

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

**macOS SIP** (issues #108, #152, #232, #360, #558): System Integrity
Protection strips `DYLD_INSERT_LIBRARIES` for system executables. Bear
detects SIP at startup via `csrutil status` and falls back to wrapper
mode. Users who disable SIP can force preload mode via configuration.

**Preload conflicts with sandboxes** (issue #675): Gentoo's sandbox
also uses `LD_PRELOAD`. The two libraries can interfere with each
other, causing test failures. Bear preserves co-resident libraries but
cannot prevent all interactions.

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

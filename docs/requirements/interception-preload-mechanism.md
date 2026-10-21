---
title: LD_PRELOAD-based command interception
status: proposed
---

## Intent

With the preload mechanism, Bear injects a shared library into every
process the build starts, and so sees every program execution during
the build. The user does not change the build system or install
compiler wrappers; any build tool that spawns compilers works. Linux and
the BSDs use `LD_PRELOAD`; macOS uses `DYLD_INSERT_LIBRARIES`. Which
mechanism runs is owned by `interception-mechanism-selection`, and so
is the promise that interception leaves the build's output alone.

## Acceptance criteria

- **Given** a build that launches `cc -c a.c` through an `exec` family
  call, `cc -c b.c` through `posix_spawn`, `cc -c c.c` through `popen`,
  and `cc -c d.c` through `system`, **when** Bear finishes, **then** all
  four compilations appear in the database (Android is an exception,
  see Known limitations).
- **Given** a build step that clears its environment (`env -i cc -c
  test.c`) and launches the compiler through a call that takes an
  explicit environment, **when** the build runs, **then** the preload
  library restores its variables in the child, and the compilation
  appears in the database.
- **Given** a build that runs `cp`, `mkdir`, and `cc -c test.c`,
  **when** the build runs, **then** all three executions are reported.
  Which reported executions become entries is owned by
  `recognition-compiler-names`.
- **Given** `LD_PRELOAD=/usr/lib/libsandbox.so` set before Bear starts,
  **when** the build runs, **then** the preload variable holds Bear's
  library first, followed by `/usr/lib/libsandbox.so`, and both stay in
  the child processes.
- **Given** a build whose last compiler reports just before the build
  exits, **when** that report races the end of interception, **then**
  it is still delivered and the last compilation appears in the
  database (issue #704).
- **Given** the preload mechanism on a macOS host with System Integrity
  Protection disabled, **when** the build spawns a compiler, **then**
  `DYLD_INSERT_LIBRARIES` is set in the child processes and compiler
  invocations are intercepted as on Linux.

## Non-functional constraints

- Platform: Linux (including Android) and the BSDs use `LD_PRELOAD`;
  macOS uses `DYLD_INSERT_LIBRARIES`. Not available on Windows, nor on
  macOS while System Integrity Protection is enabled.
- Bear finds the preload library at run time, relative to where it is
  installed, not at a path fixed when Bear was built (issues #648, #649,
  #597, #582).

## Known limitations

**Statically linked executables**: a statically linked executable does
not load the dynamic linker, so the preload library cannot intercept it.

**Environment stripped before `execvp`** (and other `exec` variants
without an explicit environment argument): the preload library restores
the preload variable only when the call passes an explicit environment.
If the build removes `LD_PRELOAD` from its environment and then spawns
children via `execvp`, grandchild processes may not be intercepted.

**Wrong ELF class during cross-compilation** (issue #236): the preload
library is built for the host architecture. When the build runs a
cross-compiler built for a different architecture, the dynamic linker
rejects the library with "wrong ELF class". The build still completes,
but those commands are not intercepted.

**glibc symbol-version mismatch in cross-compilation** (discussion
#707): the preload library is linked against the host's glibc. Loaded
into a compiler that runs against an older-glibc sysroot, it may need a
newer glibc symbol version. That invocation fails with a "version not
found" error and the command is not recorded.

**macOS SIP** (issue #558): System Integrity Protection strips
`DYLD_INSERT_LIBRARIES` for protected executables, so the preload
library cannot intercept them. What Bear does when the preload
mechanism is selected there is owned by
`interception-mechanism-selection`.

**Preload conflicts with sandboxes** (issue #699): a co-resident
`LD_PRELOAD` sandbox library that hooks the same `exec` family can
reset its own preload variable further down the exec chain and drop
Bear's entry. A grandchild past that point is not intercepted.

**`popen` and `system` on Android**: on Android these two calls are not
intercepted, so a compiler launched through them is missing from the
database. The `exec` family is intercepted as on other Linux systems.

**`posix_spawn` on older Android API levels**: Android has `posix_spawn`
and `posix_spawnp` from API level 28. A Bear built for an older API
level does not intercept them, even on a newer device, so a compiler
launched through them is missing from the database.

**Affects all child processes** (issue #444): the preload variable
applies to every process the build spawns, not only compilers. This can
disturb tools that are sensitive to preloaded libraries.

## Rationale

- [Linker selection for the preload library](../rationale/preload-linker-selection.md)
- [Preload chains to co-resident preload libraries](../rationale/preload-chains-to-co-resident-libraries.md)
- [How the build finds the functions the preload library wraps](../rationale/preload-symbol-probes.md)

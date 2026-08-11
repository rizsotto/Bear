---
title: Wrapper-based command interception
status: implemented
---

## Intent

When the user runs `bear -- make` in wrapper mode (which mode runs and
when is owned by `interception-mode-selection`), Bear intercepts
compiler invocations by placing wrapper executables on PATH ahead of the
real compilers. The build system invokes the wrapper instead of the real
compiler; the wrapper reports the execution to Bear and then forwards
the call to the real compiler. The build completes normally and the user
gets a compilation database without modifying the build system.

## Acceptance criteria

- Compiler invocations are intercepted and appear in the compilation
  database
- The build process completes normally -- interception does not alter
  build output or exit codes
- Bare compiler names in environment variables (e.g. `CC=gcc`) are
  resolved via PATH before creating wrappers
- Explicitly configured compilers select what is wrapped: when none is
  configured for interception, compilers are discovered by scanning
  PATH; when at least one is, exactly those are wrapped and the scan is
  skipped. Entries excluded from the database do not count as
  configured for interception. Compilers named in environment variables
  are wrapped either way
- The reported execution names the real compiler by its absolute path,
  never the wrapper path
- The `.bear/` directory is created in the current working directory
- The `.bear/` directory is cleaned up when Bear exits
- On Windows, executable name lookup is case-insensitive and extension-
  insensitive (`cl`, `cl.exe`, and `CL.EXE` all match)
- Reporting failures do not affect the build

## Non-functional constraints

- Must not alter build output or exit codes
- Must handle concurrent builds (parallel make) without losing reports
- Platform: works on all supported platforms (Linux, macOS, FreeBSD,
  Windows)
- The wrapper binary path must be discoverable at runtime, not baked in
  at build time (issue #668)
- The `.bear/` directory name is deterministic so that paths written
  during `./configure` survive into the `make` phase

## Known limitations

**Only intercepts known compilers**: Unlike preload mode, which
intercepts all `exec` calls regardless of the executable, wrapper mode
only intercepts compilers that Bear knows about. If the build uses a
compiler that is not in a recognized environment variable or not on
PATH at Bear startup time, it will not be intercepted.

**PATH ordering conflicts** (issue #445): The `.bear/` directory must
be first in PATH. If another tool (e.g. ccache's masquerade directory)
is also first in PATH, the ordering can cause conflicts. See
`interception-wrapper-recursion` for the specific ccache recursion
problem and its solution.

**Wrapper directory lifetime** (issue #654): If the user runs
`bear -- ./configure` and `bear -- make` as separate commands, the
`.bear/` directory is cleaned up after `./configure` exits. The
Makefile may have recorded paths into `.bear/` (e.g. as the compiler
path), causing "No such file or directory" errors during `make`. The
workaround is to combine both steps under a single Bear invocation:
`bear -- sh -c './configure && make'`. Using separate Bear invocations
will always fail because the directory is removed when the first exits.

**Cross-compilers may not be discovered** (issue #561): Bear discovers
compilers from environment variables and common names. Cross-compilers
with unusual names (e.g. `arm-none-eabi-gcc`) are only intercepted if
they appear in `CC`, `CXX`, or similar variables.

## Testing

Given a project with a single C source file:

> When the user configures wrapper mode and runs `bear -- cc -c test.c`,
> then `compile_commands.json` is created with one entry for `test.c`,
> and the compiler path in the entry is an absolute path to the real
> compiler (not `.bear/cc`),
> and the build exit code is preserved.

Given `CC=gcc` (a bare name) or `CC=/usr/bin/gcc` (an absolute path) in
the environment:

> When the user runs `bear -- make`,
> then Bear resolves the bare name via PATH to `/usr/bin/gcc` (the
> absolute path needs no resolution),
> creates `.bear/gcc` as the wrapper in both cases,
> and the compilation database entry names `/usr/bin/gcc` as the
> compiler.

Given a wrapper that cannot reach the collector:

> When the build invokes the wrapper for `cc -c test.c`,
> then the wrapper still launches the real compiler,
> the compiler's output is produced,
> and the build's exit code is the real compiler's exit code -- the
> failed report costs one database entry, never the build.

Given a parallel build with multiple source files:

> When the user runs `bear -- make -j4` with wrapper mode,
> then all compilations are intercepted,
> and the compilation database contains one entry per source file.

Given a build that fails partway through:

> When the user runs `bear -- make` and one compilation fails,
> then Bear's exit code matches the build's exit code,
> and the compilation database still contains entries for all attempted
> compilations.

Given a Windows build with `CC=cl` (no `.exe` extension):

> When Bear sets up wrapper mode and the build compiles `test.c`,
> then the wrapper directory contains exactly one wrapper for `cl` --
> `cl`, `cl.exe`, and `CL.EXE` all match it, case- and extension-
> insensitively --
> and the database entry names the real `cl.exe` by absolute path.

Given a build where the compiler is not in any environment variable:

> When the build script directly invokes `/opt/custom/bin/mycc -c test.c`
> without setting `CC`,
> then the invocation is **not** intercepted in wrapper mode
> (this is a known limitation -- preload mode would catch it).

Given a successful build:

> When `bear -- make` completes,
> then the `.bear/` directory is removed automatically,
> and no wrapper artifacts remain in the working directory.

## Notes

- Related requirement: `interception-preload-mechanism` (alternative
  interception mode using `LD_PRELOAD`).
- Related requirement: `interception-wrapper-recursion` (ccache
  recursion prevention in wrapper mode).
- The default mode per platform and the configuration override are
  owned by `interception-mode-selection`.

## Rationale

- [Hard links, deterministic directory, resolve-at-startup](../rationale/wrapper-mode-design-decisions.md) -
  why each wrapper-setup choice was made.

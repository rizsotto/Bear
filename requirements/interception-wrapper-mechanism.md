---
title: Wrapper-based command interception
status: implemented
---

## Intent

When the user runs `bear -- make` on a system where `LD_PRELOAD` is not
available (macOS with SIP, Windows) or not desired, Bear intercepts
compiler invocations by placing wrapper executables on PATH ahead of the
real compilers. The build system invokes the wrapper instead of the real
compiler; the wrapper reports the execution to Bear and then forwards
the call to the real compiler. The build completes normally and the user
gets a compilation database without modifying the build system.

## Implementation details

### Activation

Wrapper mode is the **default** on macOS and Windows. On Linux it is
available but not the default (preload mode is preferred because it
intercepts all exec calls without needing to know compiler names in
advance).

The user can force wrapper mode via the configuration file:

```yaml
schema: "4.1"
intercept:
  mode: wrapper
```

There is no CLI flag for mode selection -- it is configuration-only.

### Setup phase

When Bear starts, before launching the build command:

1. **Discovers compilers**: Bear scans the process environment for
   compiler variables (`CC`, `CXX`, `CPP`, `FC`, `AR`, `AS`, `RUSTC`,
   and others -- the full list matches Make implicit variables and Cargo
   program keys). For each variable, if the value is a bare name (e.g.
   `CC=gcc`), Bear resolves it to an absolute path via PATH lookup
   (`which`). If the value is already an absolute path, it is used
   directly. When no compiler env vars are set and no explicit compiler
   list is provided via configuration, Bear also scans PATH for known
   compiler names and creates wrappers for those found.

2. **Creates a wrapper directory**: Bear creates a `.bear/` directory in
   the current working directory. This directory is deterministic (not a
   random temp dir) so that paths baked into Makefiles during
   `bear -- ./configure` survive across separate `bear -- make` runs.

3. **Creates wrapper executables**: For each discovered compiler, Bear
   creates a hard link in `.bear/` pointing to the `bear-wrapper`
   binary. For example, `.bear/gcc` is a hard link to `bear-wrapper`.
   If hard linking fails (e.g. on overlay filesystems in containers),
   Bear falls back to a file copy. On Windows, copies are always used.

4. **Writes a configuration file**: Bear writes `.bear/wrappers.cfg`
   containing the mapping from wrapper names to real compiler paths and
   the TCP collector address.

5. **Modifies PATH**: Bear prepends `.bear/` to the front of PATH so
   that the wrappers are found before the real compilers.

6. **Updates compiler variables**: Bear updates `CC`, `CXX`, etc. to
   point to the wrapper paths in `.bear/`, so build systems that use
   these variables invoke the wrapper directly.

### Wrapper execution

When the build system invokes a compiler (e.g. `gcc -c test.c`), the
shell finds `.bear/gcc` first in PATH. The wrapper binary:

1. Captures its own invocation (executable path, arguments, working
   directory).
2. Reads `.bear/wrappers.cfg` and looks up `argv[0]` to find the real
   compiler's absolute path.
3. Replaces the wrapper path with the real compiler's absolute path in
   the captured execution.
4. Reports the execution to the TCP collector.
5. Spawns the real compiler with the original arguments, forwarding
   signals and preserving the exit code.

The wrapper **always reports an absolute path** for the compiler because
it resolves the real compiler during the setup phase. This differs from
preload mode, which reports whatever path the build system used.

### Reporting

Reporting uses the same mechanism as preload mode (see
`interception-preload-mechanism`): a fresh TCP connection per execution,
Length-Value wire format, dynamically assigned loopback port. Reporting
failures do not affect the build.

### Cleanup

When Bear exits, the `.bear/` directory is removed automatically via a
`Drop` implementation on the managed directory handle.

## Acceptance criteria

- Compiler invocations are intercepted and appear in the compilation
  database
- The build process completes normally -- interception does not alter
  build output or exit codes
- Bare compiler names in environment variables (e.g. `CC=gcc`) are
  resolved via PATH before creating wrappers
- The wrapper reports the real compiler's absolute path, not the wrapper
  path
- The `.bear/` directory is created in the current working directory
- The `.bear/` directory is cleaned up when Bear exits
- On Windows, executable name lookup is case-insensitive and extension-
  insensitive (`cl`, `cl.exe`, and `CL.EXE` all match)
- Reporting failures do not affect the build
- Signal forwarding works -- if the user sends SIGINT to the build,
  the wrapper forwards it to the real compiler and returns its exit code

## Non-functional constraints

- Must not alter build output or exit codes
- Must handle concurrent builds (parallel make) -- each wrapper opens
  its own TCP connection for reporting
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

**Wrapper path must be discoverable** (issue #668): The `bear-wrapper`
binary path was previously baked in at build time via a compile-time
constant. This broke prebuilt packages (conda-forge, scoop, system
packages) where the install path differs from the build path. The
wrapper path must be resolved relative to the `bear` binary at runtime.

**Cross-compilers may not be discovered** (issue #561): Bear discovers
compilers from environment variables and common names. Cross-compilers
with unusual names (e.g. `arm-none-eabi-gcc`) are only intercepted if
they appear in `CC`, `CXX`, or similar variables.

## Design decisions

**Hard links, not symlinks**: Wrappers are hard links to `bear-wrapper`,
not symlinks. This is because tools like ccache detect symlinks to
themselves and skip them, but do not detect hard links. While this
creates the ccache recursion problem described in
`interception-wrapper-recursion`, it ensures that ccache does not skip
Bear's wrapper entirely.

**Deterministic directory name**: Using `.bear/` in the current working
directory (rather than a random temp dir) ensures that paths recorded
during `./configure` remain valid during `make`, provided both run
under the same Bear invocation. This was a deliberate choice after
issue #654 showed that temp directories break multi-step builds.

**Compiler resolution at startup**: Bear resolves bare compiler names
to absolute paths once at startup, not each time the wrapper is
invoked. This avoids repeated PATH lookups and ensures the wrapper
configuration is self-contained. The tradeoff is that if PATH changes
during the build, the wrapper still uses the originally resolved path.

## Testing

Given a project with a single C source file:

> When the user configures wrapper mode and runs `bear -- cc -c test.c`,
> then `compile_commands.json` is created with one entry for `test.c`,
> and the compiler path in the entry is an absolute path to the real
> compiler (not `.bear/cc`),
> and the build exit code is preserved.

Given `CC=gcc` (a bare name) in the environment:

> When the user runs `bear -- make`,
> then Bear resolves `gcc` via PATH to an absolute path
> (e.g. `/usr/bin/gcc`),
> creates `.bear/gcc` as a wrapper,
> and the compilation database contains `/usr/bin/gcc` as the compiler.

Given a build that uses an absolute compiler path (`CC=/usr/bin/gcc`):

> When the user runs `bear -- make`,
> then Bear creates `.bear/gcc` pointing to `/usr/bin/gcc`,
> and the wrapper intercepts the invocation correctly.

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

> When Bear looks up `cl` in the wrapper configuration,
> then it matches case-insensitively and without requiring the `.exe`
> extension,
> and the wrapper is created correctly.

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
- The wrapper binary is a separate Rust binary (`bear/src/bin/wrapper.rs`)
  that is built alongside the main `bear` binary.
- GitHub issues #686, #681 cover Windows/MSYS2 wrapper mode improvements.
- GitHub issue #609 describes user confusion about when to use wrapper
  vs. preload mode. The default selection (wrapper on macOS/Windows,
  preload on Linux) handles the common case; the configuration file
  covers the rest.

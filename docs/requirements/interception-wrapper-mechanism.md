---
title: Wrapper-based command interception
status: implemented
---

## Intent

With the wrapper mechanism, Bear puts wrapper executables, named after
the compilers, in a wrapper directory first on PATH, and points the
compiler environment variables (`CC`, `CXX`, and similar) at them. The
build runs the wrapper instead of the real compiler. The wrapper reports
the execution to Bear and runs the real compiler. The user gets a
compilation database without changing the build system. Which mechanism
runs is owned by `interception-mechanism-selection`, and so is the
promise that interception leaves the build's output alone.

## Acceptance criteria

- **Given** a build that runs `cc -c test.c`, **when** Bear finishes,
  **then** the database holds one entry for `test.c`, and the entry
  names the real compiler by its absolute path, never the wrapper path.
- **Given** a wrapped compiler that runs another wrapped compiler by its
  bare name, **when** the build runs, **then** the wrapper directory is
  still first on PATH for that process, and the inner call is
  intercepted too.
- **Given** `CC=gcc` (a bare name) or `CC=/usr/bin/gcc` (an absolute
  path), **when** the build runs, **then** Bear resolves the bare name
  via PATH to `/usr/bin/gcc`, wraps `gcc` in both cases, and the entry
  names `/usr/bin/gcc` as the compiler. Values that carry flags are
  owned by `interception-cc-env-var-flags`.
- **Given** a configuration that lists no compiler for interception
  (entries excluded from the database do not count), **when** Bear
  starts, **then** it scans PATH and wraps every compiler it recognizes
  there. Which names it recognizes, cross-compilation prefixes included,
  is owned by `recognition-compiler-names`.
- **Given** a configuration that lists at least one compiler for
  interception, **when** Bear starts, **then** it wraps exactly the
  listed paths, whatever their names (an unusual cross-compiler
  included), and skips the PATH scan.
- **Given** a compiler named in a compiler environment variable,
  **when** Bear starts, **then** it is wrapped, whether or not the
  configuration lists compilers.
- **Given** a build that runs `/opt/custom/bin/mycc -c test.c` directly,
  with `mycc` not named in a compiler variable and not listed in the
  configuration, **when** Bear finishes, **then** that invocation is not
  intercepted.
- **Given** a build under Bear, **when** the build runs, **then** the
  wrapper directory `.bear/` exists in the working directory, and
  **when** Bear exits, **then** it is removed and no wrapper artifacts
  remain.
- **Given** two Bear runs in the same working directory, **when** each
  runs its build, **then** each uses the same wrapper directory path.
- **Given** one Bear invocation that runs `./configure` and then `make`
  (`sh -c './configure && make'`), **when** `./configure` records a
  path into the wrapper directory, **then** that path stays valid for
  `make`.
- **Given** a Windows build with `CC=cl` (no `.exe` extension), **when**
  the build compiles `test.c`, **then** the wrapper directory holds
  exactly one wrapper for `cl`, which `cl`, `cl.exe`, and `CL.EXE` all
  match, and the entry names the real `cl.exe` by absolute path.

## Non-functional constraints

- Platform: works on all supported platforms (Linux, macOS, FreeBSD,
  Windows).
- Bear finds the wrapper binary at run time, not at a path fixed when
  Bear was built (issue #668).

## Known limitations

**Only wrapped compilers are intercepted**: the wrapper mechanism sees
only the compilers it wrapped at startup, and only when the build finds
them through PATH or a compiler variable. A compiler that is not on
PATH, not in a compiler variable, and not listed in the configuration
is not intercepted.

**Unrecognized compiler names** (issue #561): a compiler whose name
Bear does not recognize is skipped by the PATH scan. It is wrapped only
when a compiler variable names it, or when the configuration lists it.
Listing it turns the PATH scan off, so the other compilers must be
listed too.

**PATH ordering conflicts** (issue #445): the wrapper directory must be
first in PATH. A masquerade directory such as ccache's can conflict
with it. Resolving past masquerade wrappers is owned by
`interception-wrapper-recursion`.

**Wrapper directory lifetime** (issue #654): Bear removes the wrapper
directory when it exits. If the configure step runs under Bear and a
later build step runs without Bear, a path recorded into the directory
(for example as the compiler path) no longer exists, and the build
fails with "No such file or directory". Running every step that uses
the recorded path under Bear avoids this.

## Rationale

- [Wrapper mechanism: hard links, deterministic directory, resolve-at-startup](../rationale/wrapper-mode-design-decisions.md)

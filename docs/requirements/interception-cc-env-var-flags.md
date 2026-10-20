---
title: Handle compiler env vars that contain flags
status: implemented
---

## Intent

Compiler environment variables often carry the compiler and a simple
trailing flag or two, as in `CC="gcc -std=c11"` or
`CXX="clang++ -stdlib=libc++"`. This is a GNU Make convention seen in
configure scripts, Makefiles, and Dockerfiles. When the user runs a
build under Bear with the wrapper mechanism and such a value, Bear wraps
the real program and rewrites the variable so the build still gets the
flags.

## Acceptance criteria

- **Given** `CC="fake-cc -std=c11"` and `fake-cc` on PATH, **when** Bear
  sets up the wrapper mechanism, **then** the wrapper for `fake-cc` runs
  the real compiler, and the build sees `CC` as
  `<wrapper path> -std=c11`.
- **Given** `CC="/abs/path/my-gcc -m32"`, **when** Bear sets up the
  wrapper mechanism, **then** the wrapped program is `/abs/path/my-gcc`,
  and the rewritten value keeps `-m32`.
- **Given** a C source file, a build script that runs `$CC -c test.c`,
  and `CC="<compiler> -DBEAR_TEST_FLAG=1"`, **when** Bear finishes,
  **then** the build succeeds, and the database holds exactly one entry
  whose arguments contain `-DBEAR_TEST_FLAG=1`.
- **Given** a value with flags, **when** Bear rewrites it, **then** the
  new value is the wrapper path and the original flag tokens, joined by
  single spaces, with no shell quoting added. With no flags, the new
  value is the wrapper path alone, with no quoting added.
- **Given** a value with quotes, shell metacharacters, or a command
  substitution, **when** Bear reads it, **then** none of these has a
  special meaning: the value is split on whitespace only.
- **Given** a ccache masquerade directory on PATH and a real compiler
  past it, **when** `CC="gcc -std=c11"`, **then** the wrapper runs the
  real compiler past the masquerade directory, and the rewritten `CC`
  still holds `-std=c11`. Masquerade handling is owned by
  `interception-wrapper-recursion`.
- **Given** a Unix-like shell on Windows with forward-slash paths,
  **when** `CC="C:/tools/fake-cc.exe -DBEAR_TEST=1"`, **then** a wrapper
  is registered for `fake-cc.exe`, and the rewritten `CC` still holds
  `-DBEAR_TEST=1`.
- **Given** `CC=""` or `CC="   "`, **when** Bear sets up the wrapper
  mechanism, **then** the variable is not rewritten, and a warning is
  logged.

A value without flags, or an unset variable, follows the ordinary
resolution owned by `interception-wrapper-mechanism`.

## Non-functional constraints

- Platform support: Unix and Unix-like environments: Linux, macOS, BSD,
  and the Unix-like shells hosted on Windows (MSYS2, Git Bash, WSL).

## Known limitations

- No effect on a native Windows build: native Windows build tooling
  (MSBuild, `nmake`, `cmd`, PowerShell) does not read `CC`/`CXX` from
  the environment.

## Rationale

- [Whitespace split, no shell parser](../rationale/compiler-env-flag-parsing.md)

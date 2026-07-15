---
title: Handle compiler env vars that contain flags
status: implemented
---

## Intent

Compiler environment variables often carry not just the compiler path but
a simple trailing flag or two. This is a GNU Make convention and shows
up in configure scripts, Makefiles, and Dockerfiles:

- `CC="gcc -std=c11"`
- `CXX="clang++ -stdlib=libc++"`
- `CC="/usr/local/bin/gcc -m32"`

When the user runs `bear -- make` with a value of this shape, Bear's
wrapper mode registers a wrapper for the real program and rewrites the
env var so the build still receives the flags.

Anything more elaborate -- flags with embedded whitespace, shell
quoting, metacharacters, command substitutions -- belongs in `CFLAGS` /
`CXXFLAGS` / `LDFLAGS`. Bear splits the env var value on whitespace
and goes no further. The man page points users at `CFLAGS` for
anything that does not fit that shape.

## Acceptance criteria

- Wrapper mode splits a compiler env var into `(program, flags)` on
  whitespace before resolution.
- `CC="gcc -std=c11"` resolves `gcc` via PATH and registers a wrapper
  (existing masquerade handling still applies to the program token,
  see `interception-wrapper-recursion`).
- `CC="/usr/bin/gcc -m32"` extracts `/usr/bin/gcc` as the program.
- The rewritten env var value is the wrapper path followed by the
  original flag tokens, joined with single spaces and without shell
  quoting. When there are no flags, the override is the wrapper path
  as a bare string, with no quoting added.
- An empty or whitespace-only value is skipped with a warning.

## Non-functional constraints

- Platform support: Unix and Unix-like environments -- Linux, macOS,
  BSD, and the Unix-like shells hosted on Windows (MSYS2, Git Bash,
  WSL). The `CC`/`CXX` convention is a Unix / GNU Make inheritance.
- A flagless value (a bare name such as `CC=gcc` or an absolute path
  such as `CC=/usr/bin/gcc`) and an unset variable follow the ordinary
  wrapper-mode resolution owned by `interception-wrapper-mechanism`;
  the whitespace split is observable only when flag tokens are present.

## Known limitations

- No observable effect on a native Windows build: native Windows build
  tooling (MSBuild, `nmake`, `cmd`, PowerShell) does not read
  `CC`/`CXX` from the environment.

## Testing

### Unit tests (in `crates/bear/src/environment.rs`)

Given a wrapper-mode setup with a fake compiler on PATH:

> When `CC="fake-cc -std=c11"`,
> then the wrapper config is keyed by the compiler basename and points
> at the real compiler,
> and the override value for `CC` equals
> `"<wrapper_path> -std=c11"`.

Given a wrapper-mode setup with a fake compiler at an absolute path:

> When `CC="/abs/path/my-gcc -m32"`,
> then the resolved program is `/abs/path/my-gcc`,
> and the override preserves `-m32`.

Given a wrapper-mode setup with no flags (regression guard):

> When `CC=/usr/bin/gcc`,
> then the override value equals the wrapper path verbatim (no shell
> quoting introduced).

Given a whitespace-only value:

> Then `CC` is skipped.

Given a ccache masquerade directory on PATH and a real compiler past it:

> When `CC="gcc -std=c11"`,
> then the wrapper config entry points at the real compiler past the
> masquerade directory,
> and the override for `CC` still contains `-std=c11`.

Given a Unix-like shell on Windows producing forward-slash paths:

> When `CC="C:/tools/fake-cc.exe -DBEAR_TEST=1"`,
> then the wrapper is registered for `fake-cc.exe`,
> and the override for `CC` still contains `-DBEAR_TEST=1`.

### Integration test (`tests/integration/tests/cases/intercept.rs`)

Given a C source file and a build script that runs `$CC -c test.c`:

> When the user runs `bear` in wrapper mode with
> `CC="<compiler> -DBEAR_TEST_FLAG=1"`,
> then the build succeeds,
> and `compile_commands.json` has exactly one entry,
> and the `arguments` array contains `-DBEAR_TEST_FLAG=1`.

## Notes

- Issue #686 -- bare-name CC resolution.
- Related: `interception-wrapper-mechanism` -- the wrapper mode this
  feature extends.
- Related: `interception-wrapper-recursion` -- masquerade handling of
  the program token.
- Related: `output-env-derived-flags` -- folding include/option env
  vars into entries, a separate output-side mechanism.

## Rationale

- [Whitespace split, no shell parser](../rationale/compiler-env-flag-parsing.md) -
  why the env-var value is split on whitespace instead of shell-parsed.

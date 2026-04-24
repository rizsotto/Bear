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

## Scope

This feature targets Unix and Unix-like environments: Linux, macOS,
BSD, and the Unix-like shells hosted on Windows (MSYS2, Git Bash,
WSL). The `CC`/`CXX` convention is a Unix / GNU Make inheritance.

Native Windows build tooling picks compilers through different
channels. MSBuild resolves the compiler from project files and the
installed Visual Studio / Build Tools toolchain; `nmake` does not
inherit `CC`/`CXX` from the environment by default (its predefined
macros are read from makefiles or passed with `/E`, not from the
process environment); `cmd` and PowerShell have no `CC` convention
of their own. None of these pathways read `CC`/`CXX` environment
variables, so on a native Windows build the feature has no
observable effect. The Windows use case Bear targets is running a
Unix-convention build (e.g. `make`) under one of the Unix-like
shells named above -- the same scenario the rest of Bear's wrapper
mode is built for on Windows.

## Background

`resolve_program_path` in `bear/src/intercept/environment.rs` used to
pass the entire env var value to `PathBuf::from()` / `which::which_in()`.
Both treat the whole string as a single filename, so any value with
trailing flags failed to resolve and the env var was skipped with a
warning. The build then ran the real compiler directly and the
compilation database ended up missing entries.

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
  as a bare string, byte-identical to today's no-flags behavior.
- Empty or whitespace-only values are skipped gracefully (same
  warn-and-skip behavior as before this feature; log message text
  differs).

## Implementation details

### Parsing helper

A private `parse_program_env_value(value: &str) -> Option<(String,
Vec<String>)>` lives next to `create_as_wrapper`. It calls
`str::split_whitespace` and returns the first token plus the rest;
empty input yields `None`. No shell parser is involved.

`resolve_program_path` is not touched. It already owns the
masquerade-wrapper contract (`interception-wrapper-recursion`);
widening its signature to return flags would expand the blast radius
for no gain. The call site does the split and passes only the program
token in.

### Override reconstruction

- No flags -> `wrapper_path.to_string_lossy()`.
- With flags -> `"<wrapper_path> <flag1> <flag2> ..."` (single-space
  join, no quoting).

Shell quoting is deliberately not introduced on the output side. `$CC`
expansion in a shell script does not re-apply quote removal on the
expanded text, so any quoting Bear added would leak literal quotes into
argv. Make recipes handed to `sh -c` would re-interpret the added
quoting as a new layer. Space-joining matches what both paths expect;
if that is not enough for a given flag, it belongs in `CFLAGS`.

### Files changed

- `bear/src/intercept/environment.rs`: new `parse_program_env_value`
  helper and rewritten env-var loop in `create_as_wrapper`.
- `man/bear.1.md` (and regenerated `bear.1`): TROUBLESHOOTING paragraph
  pointing users at `CFLAGS` for anything beyond whitespace-separated
  tokens.

## Non-functional constraints

- No shell parser or subprocess is involved.
- Behavior for today's common inputs (`CC=gcc`, `CC=/usr/bin/gcc`,
  unset) does not change.

## Testing

### Unit tests (in `bear/src/intercept/environment.rs`)

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

Given a Unix-like shell on Windows producing forward-slash paths (see
Scope):

> When `CC="C:/tools/fake-cc.exe -DBEAR_TEST=1"`,
> then the wrapper is registered for `fake-cc.exe`,
> and the override for `CC` still contains `-DBEAR_TEST=1`.

### Integration test (`integration-tests/tests/cases/intercept.rs`)

Given a C source file and a build script that runs `$CC -c test.c`:

> When the user runs `bear` in wrapper mode with
> `CC="<compiler> -DBEAR_TEST_FLAG=1"`,
> then the build succeeds,
> and `compile_commands.json` has exactly one entry,
> and the `arguments` array contains `-DBEAR_TEST_FLAG=1`.

## Notes

### Alternatives considered and rejected

**POSIX shell-word parsing (`shell_words::split`).** Handles quoted
program names and paths with spaces. Rejected: the spec explicitly
redirects such cases to `CFLAGS`, and the parser's quote-aware edge
cases (malformed quotes, backslash-as-escape on Windows paths) created
test surface area with no matching user need. Whitespace splitting
matches the shape the spec actually supports and sidesteps all of
this.

**Full shell expansion (`sh -c 'command -v "$CC"'`).** Would resolve
everything the way the build system would. Rejected: requires spawning
a shell, and the common Make/Autoconf convention is effectively
`$(firstword $(CC))` plus the rest as flags, which is what whitespace
splitting gives us directly.

### Related

- Issue #686 -- bare-name CC resolution.
- `interception-wrapper-mechanism`, `interception-wrapper-recursion`.

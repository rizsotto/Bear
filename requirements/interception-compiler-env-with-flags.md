---
title: Handle compiler env vars that contain flags
status: proposed
---

## Problem

Compiler environment variables sometimes contain not just the compiler path
but also flags. For example:

- `CC="gcc -std=c11"`
- `CXX=clang++ -stdlib=libc++`
- `CC="/usr/local/bin/gcc -m32"`

This is a common pattern in build systems. GNU Make documents it, and many
projects use it in their configure scripts or Makefiles.

Currently `resolve_program_path` (in `bear/src/intercept/environment.rs:249`)
passes the entire env var value -- including flags -- to `PathBuf::from()` or
`which::which_in()`. Both treat the whole string as a single filename, which
fails to resolve. The env var is then skipped with a warning and the wrapper
is never created for that compiler.

## Example scenarios

### 1. Bare name with flags

```
CC="gcc -std=c11" make
```

`resolve_program_path` receives `gcc -std=c11`, tries to find an executable
with that literal name, fails, and skips it.

### 2. Absolute path with flags

```
CC="/usr/bin/gcc -m32" make
```

`PathBuf::from("/usr/bin/gcc -m32").is_absolute()` returns true, so the
function returns the path as-is. But `/usr/bin/gcc -m32` is not a real file.
The wrapper registration then fails or registers a non-existent path.

### 3. Quoted values

```
CC='"gcc" -Wall'
```

The value arrives with embedded quotes. The program name must be extracted
from inside the quotes.

## Integration test plan

Set up:
- A test environment with a single source file `test.c`.
- A shell build script that compiles `test.c` by invoking `$CC`.

Run Bear in wrapper mode with `CC` set to a compiler name followed by a
flag (for example `gcc -std=c11`). The build script runs `$CC -c test.c`,
which expands to `gcc -std=c11 -c test.c`.

Verify:
- The build succeeds (Bear did not skip the env var and did not confuse
  the flag for part of the compiler name).
- The output compilation database contains exactly one entry for `test.c`.
- The entry's arguments include the `-std=c11` flag that came from the
  env var, proving the flag was preserved through interception.

Additional cases to cover in separate tests or parameterization:
- Absolute path with flags: `CC="/usr/bin/gcc -m32"`.
- Quoted program name: `CC='"gcc" -Wall'`.
- Multiple flags: `CC="gcc -std=c11 -Wall -O2"`.
- Empty / whitespace-only values should be skipped gracefully without
  aborting the build.

## Acceptance criteria

- [ ] `CC="gcc -std=c11"` resolves `gcc` via PATH and registers a wrapper
- [ ] `CC="/usr/bin/gcc -m32"` extracts `/usr/bin/gcc` and passes it through
- [ ] Flags from the env var value are preserved in the environment override
      so the build command still receives them
- [ ] Quoted program names (`CC='"gcc" -Wall'`) are handled correctly
- [ ] Empty or whitespace-only values are still skipped gracefully

## Solutions

### Option A: Extract program name in `resolve_program_path`

Add a parsing step at the top of `resolve_program_path` that splits the value
into program and trailing flags. The function returns only the resolved
program path. The caller then reconstructs the env var override by replacing
the original program portion with the wrapper path, preserving the flags.

Parsing rules (in order):
1. If the value starts with `"` or `'`, extract the quoted portion as the
   program name.
2. Otherwise, split on the first whitespace. The first token is the program
   name, the rest are flags.

Changes needed:

- `resolve_program_path` returns `Option<(PathBuf, Option<String>)>` where
  the second element is the trailing flags (if any).
- The env var registration loop in `create_as_wrapper` (line 173) uses the
  flags to construct the override: `format!("{} {}", wrapper_path, flags)`.

**Complexity**: Low. Isolated change in one function plus its call site.
**Alignment**: Good. Keeps the resolution logic centralized. The parsing is
straightforward and does not require a shell parser.

### Option B: Use `shell_words` crate (already a dependency)

Use `shell_words::split()` to parse the env var value as a shell would. The
first element is the program name, the rest are flags. Reconstruct with
`shell_words::join()`.

Changes needed:

- Same as Option A, but the parsing step uses `shell_words::split()` instead
  of manual splitting.

**Complexity**: Low. `shell_words` is already in `Cargo.toml`.
**Alignment**: Good. Handles edge cases (nested quotes, escaped spaces in
paths) that manual parsing would miss. This is the same approach the semantic
analysis layer uses for argument parsing.

### Option C: Full shell expansion

Run the value through the shell (`sh -c "command -v $CC"`) to resolve it
exactly as the build system would.

**Complexity**: High. Requires spawning a shell process, handling errors,
and dealing with platform differences.
**Alignment**: Poor. Over-engineered for this use case. The env var format
is simple enough that shell-level expansion is not needed.

## Recommendation

**Option B** (`shell_words`). It handles quoting edge cases correctly, the
dependency already exists, and the change is small. Option A is acceptable
if we want zero new API surface, but it will need special cases for quotes
that `shell_words` handles naturally.

## Notes

- The reporter's PR (#687) included an `extract_program_from_env` function
  that handled this case with manual quote parsing. We chose not to include
  it in the initial fix to keep the scope focused, but the problem is real.
- Related issue: #686.

<!-- Diataxis type: how-to -->

# clangd has no compile command for a header

clangd already guesses a compile command for a header with no entry of
its own, by borrowing the flags of a compiled source file nearby, and for
many projects that guess is good enough as it stands. When it is not -
the borrowed flags are wrong for this header, or there is no source
nearby to borrow from - turn on header-entry synthesis in Bear's
configuration file:

```yaml
schema: "4.2"
headers:
  enabled: true
  strategy: siblings
```

This makes Bear write a real entry for the header itself, cloned from a
compiled source's arguments with the source path swapped in for the
header's and the output flag removed, instead of leaving clangd to infer
one. The full set of `headers:` keys and their defaults is on [Configure
Bear](../../reference/configuration.md#headers).

## Telling the situations apart

Run clangd against the header directly and read the log line it prints
while loading the database:

```sh
clangd --check=path/to/header.h
```

- `Compile command from CDB is: ...` - the database already has a real
  entry for this header. This is not the problem this page is about.
- `Compile command inferred from some_source.c is: ...` - clangd found no
  entry but borrowed one from a neighboring source in the same directory.
  This is often fine as it stands; look at whether the borrowed flags
  actually apply to the header (same defines, same target) before
  changing anything.
- `Generic fallback command is: ...` - clangd found no entry and no
  neighbor to borrow from at all. This is the case synthesis exists for.

[Set up clangd for a project without CMake](clangd-setup.md) documents
these same three log lines with the fuller context of a working
database; this page only adds the header-specific reading of them.

## Synthesis is not always the difference between broken and working

In a split `include/` + `src/` project (`include/greet.h` used by
`src/greet.c`, which lives in a different directory), running with header
synthesis off still leaves clangd able to work on `include/greet.h`: it
infers a command from `src/greet.c` and logs `Compile command inferred
from src/greet.c is: ... -x c-header ...`. Turning on the
`dependency-files` strategy replaces that with a real entry, and the log
line changes to `Compile command from CDB is: ...` - but the header was
already usable before that change. Reach for synthesis when one of these
is actually true, not by default:

- the flags a neighboring source would donate are wrong for this header
  (a different target, different defines, a header shared across parts
  of the build that compile differently);
- no compiled source is in reach to donate flags from at all (the
  `Generic fallback command` case above);
- a consumer needs a real database entry rather than clangd's own
  inference - running `clang-tidy` directly against the header, or
  another tool that reads `compile_commands.json` without clangd's
  fallback logic.

## Choosing a strategy

Two strategies are available, and they reach different headers:

- **`siblings`** clones flags from a compiled source in the same
  directory as the header. It needs nothing from the build, but it
  reaches only headers that share a directory with a compiled source: in
  a split `include/` + `src/` layout it picks up a header living beside
  `.c` files in `src/`, but not one living alone in `include/`, because
  that directory has no compiled source to clone from. The flags it
  clones are approximate, borrowed from whichever compiled source
  happens to sit next to the header, not necessarily the one that
  actually includes it.
- **`dependency-files`** reads the make-style `.d` files the build
  already left on disk (from `-MMD`, `-MD`, or an equivalent flag) and
  synthesizes an entry for every header named as a prerequisite,
  wherever it lives. This is how it reaches a header in `include/` from a
  `.d` file written while compiling a source in `src/`. It needs the
  build to have actually emitted those files: look for `*.d` files next
  to the object files after a build, or check the compiler flags for
  `-MMD`/`-MD`, before choosing this strategy. A build that never passes
  one of those flags leaves nothing for `dependency-files` to read, and
  synthesis silently reaches no headers.

Start with `siblings` when the project's headers all sit next to the
sources that use them; reach for `dependency-files` when they do not, or
when `siblings`'s approximate flags are visibly wrong, and the build
already produces `.d` files.

A third strategy is conceivable: post-process an existing compilation
database and determine, for each translation unit, which header files it
actually used. That would be more accurate than cloning a neighbor's
flags, and would not require the build to have emitted dependency files.
Bear does not implement this strategy yet; external tools exist that do
this kind of post-processing.

Related: [Bear produces an empty
compile_commands.json](empty-compilation-database.md) for a database
missing more than just header entries, [Generate compile_commands.json
for a Makefile project](compile-commands-for-makefile.md), and the
[Recipes](index.md) index.

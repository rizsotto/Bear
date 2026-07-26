<!-- Diataxis type: tutorial -->

# Recover compile_commands.json from a build log

By the end of this page you will have turned a plain text build log,
with no live build behind it, into a working `compile_commands.json`,
using `bear parse-sh` instead of running anything.

This is for the build you cannot run again: a CI job that already
finished and left its log in an artifact, a machine you no longer have
access to, or a build that takes an hour when you only need the
compiler flags out of it. [Getting started with Bear](getting-started.md)
covers the other path, running a build under Bear as it happens; this
page covers reconstructing a database from text alone, after the fact.

## Get a build to reconstruct

Download Lua 5.4.8, a small, dependency-free C project, and extract it:

```sh
curl -LO https://www.lua.org/ftp/lua-5.4.8.tar.gz
tar xzf lua-5.4.8.tar.gz
cd lua-5.4.8
```

## Capture a dry run

Ask `make` to print the commands it would run, without running them,
and save that text to a file:

```sh
make -nw > build.log
```

`-n` is the dry run: `make` prints each recipe line instead of
executing it. `-w` makes `make` also print `Entering directory` and
`Leaving directory` lines around every directory it works in. Lua's
top-level `Makefile` hands the real build off to a `make` invocation in
`src/`; that inner `make` already announces its own directory by
default, and `-w` additionally names the top-level one, so the log
names every directory the build touched, not just the ones recursion
would have announced anyway.

## Turn the log into a database

```sh
bear parse-sh -i build.log
```

```
bear: warning: line 4: skipped (command substitution)
bear: warning: line 5: skipped (command substitution)
bear: warning: parse-sh: 40 command(s) parsed, 2 line(s) skipped
```

Those two warnings point at Lua's own platform-detection recipe, which
runs `` echo Guessing `uname` `` and `` make `uname` `` to pick a
default target: the backticks are shell command substitution, outside
the subset `parse-sh` understands, so it skips both lines and says why.
Neither line is a compiler invocation, so nothing is missing from the
result; a skip only costs you an entry when the skipped line was one.
The run still exits 0, because most of the log did parse.

`compile_commands.json` now holds one entry per source file `make`
would have compiled:

```sh
grep -c '"file"' compile_commands.json
```

```
34
```

and each entry looks like this:

```json
{
  "directory": "/home/you/lua-5.4.8/src",
  "file": "lapi.c",
  "arguments": [
    "gcc",
    "-std=gnu99",
    "-O2",
    "-Wall",
    "-Wextra",
    "-DLUA_COMPAT_5_3",
    "-DLUA_USE_LINUX",
    "-c",
    "-o",
    "lapi.o",
    "lapi.c"
  ],
  "output": "lapi.o"
}
```

`directory` is `src`, not the checkout root: `parse-sh` followed the
`cd src` in the top-level recipe and the `Entering directory` lines
`-w` added, the same way it would for a real recursive build.

## The same log, read somewhere else

The scenario this page is really for is a log you did not just
produce: someone hands you `build.log` from a build that ran elsewhere,
or ran an hour ago on a machine that is gone now. Copy it somewhere
that has never seen the Lua checkout, and parse it from there:

```sh
mkdir ~/handed-a-log
cp lua-5.4.8/build.log ~/handed-a-log/
cd ~/handed-a-log
bear parse-sh -i build.log
grep -m1 '"directory"' compile_commands.json
```

```
    "directory": "/home/you/lua-5.4.8/src",
```

Same 34 entries, same directory, even though `~/handed-a-log` has no
`lua-5.4.8` anywhere near it. `parse-sh` gets `directory` from the `cd`
and `Entering directory` text inside the log itself, never from where
you happen to run it; that is what makes a saved log portable. Capture
with `-nw` as a habit: it costs nothing and guarantees the log names
its own build root, including for a Makefile whose top level compiles
directly instead of only delegating, which the
[Makefile recipe](recipes/compile-commands-for-makefile.md) covers in
more detail.

## Limits of reconstructing from text

`parse-sh` reconstructs commands from what a build system chose to
print; it never observes anything running. That makes it lower fidelity
than intercepting a real build, in specific, permanent ways:

- A dry run can omit commands entirely. Recursive `make` does not
  always propagate `-n` to sub-makes, and a command that compiles a
  not-yet-generated source never gets printed in the first place.
- A bare executable name in the log, like `gcc` above, resolves against
  the environment and `PATH` you run `parse-sh` in, not the original
  build's. If the build used a different compiler on `PATH`, the
  database will point at yours instead.
- Only a documented subset of POSIX shell is understood: word splitting
  and quoting, the `;`, `&&`, `||`, `&`, and `|` separators, comments,
  redirections, `cd`, brace groups, and make's directory markers.
  Subshells, command substitution (what tripped the two lines above),
  parameter expansion, globs in the executable position, here-documents,
  and shell keywords such as `if` or `for` are all outside it and get
  skipped, loudly, rather than guessed at. See `bear parse-sh` in the
  [`bear(1)` man page][manpage] for the exact list.

Prefer `bear -- make` (see [Getting started with
Bear](getting-started.md) and the [Makefile
recipe](recipes/compile-commands-for-makefile.md)) whenever you can
actually run the build: it observes the real `exec()` calls, so it
cannot miss a command or resolve a compiler name wrong. Reach for
`parse-sh` only when running the build again is not an option.

## Related pages

- [Getting started with Bear](getting-started.md) for the tutorial that
  builds a project under Bear directly, instead of from a log.
- [Generate compile_commands.json for a Makefile
  project](recipes/compile-commands-for-makefile.md) for recursive
  builds, incremental builds, and the rest of the Make-specific detail.
- [Bear produces an empty compile_commands.json](recipes/empty-compilation-database.md)
  if a database, from either path, comes out shorter than expected.
- [Recipes](recipes/index.md) for the rest of the task pages.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

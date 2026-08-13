<!-- Diataxis type: how-to -->

# Capture a complete dry run for bear parse-sh

A dry run is only worth parsing if it prints every compiler command, so
ask make for all of them at once:

```sh
LC_ALL=C make -Bnwk | bear parse-sh
```

Each flag closes off one way the text comes up short. `-n` prints
recipes instead of running them. `-B` prints them even in a tree that is
already built. `-k` carries on past an error rather than stopping at the
first one. `-w` makes the top-level make announce its directory, which
sub-makes already do. `LC_ALL=C` keeps those announcements in the
English wording Bear matches. Three things no flag fixes: a recursion
that never descends, a build tool whose dry run prints progress instead
of commands, and a log that was already captured without them.

## A recursive make that never descends

Under `-n` make prints a recipe line instead of running it, and that
applies to a recipe that starts a make in a subdirectory too: the line
prints, the sub-make never runs, and nothing from that subdirectory
reaches the text. The exception is a line make recognizes as recursion,
which it does by the text, `$(MAKE)` or `${MAKE}` somewhere in the line,
or a leading `+`. A Makefile that spells the recursion out literally:

```
all:
	make -C sub
```

gives a dry run that stops at the top level:

```shell
$ make -nw
make: Entering directory '/home/you/project'
make -C sub
make: Leaving directory '/home/you/project'
```

`bear parse-sh` writes `[]` from that and has nothing to report:
`make -C sub` is a valid command, it is just not a compiler. Writing the
same rule as `$(MAKE) -C sub` makes the sub-make run under `-n` as well
and print its own recipes, `Entering directory` markers included.

When the Makefile is not yours to change, dry-run each subdirectory
separately and combine the results with `--append` (see [Command-line
options](../../reference/command-line.md#bear-parse-sh)):

```sh
make -nw | bear parse-sh
make -nw -C sub | bear parse-sh --append
```

`make -C` announces the directory it changed into, so the appended
entries carry the right `directory` whichever directory you run the
command from.

## An up-to-date tree, or one that stops on an error

Neither case is loud: everything Bear does get is valid text, so no line
is skipped and the run exits 0, however little reached the database. A
tree that is already built has no recipe left to print:

```shell
$ make -nw
make: Entering directory '/home/you/project'
make: Nothing to be done for 'all'.
make: Leaving directory '/home/you/project'
```

That middle line is a valid command that is not a compiler, so there is
no skip to report. `-B` (`--always-make`) treats every target as out of
date and prints every recipe.

An error stops the dry run where it would stop a real build. The
compiles make had already reached are printed, the rest never are, and
`-k` (`--keep-going`) is what gets the unaffected targets printed
anyway:

```shell
$ make -nw
make: Entering directory '/home/you/project'
make: *** No rule to make target 'gone.c', needed by 'gone.o'.  Stop.
make: Leaving directory '/home/you/project'
$ make -nwk
make: Entering directory '/home/you/project'
make: *** No rule to make target 'gone.c', needed by 'gone.o'.
gcc -c -o a.o a.c
gcc -c -o b.o b.c
make: Leaving directory '/home/you/project'
```

## Directory markers, and the locale they are printed in

Sub-makes announce their directory on their own; the top-level make
announces its own only under `-w`. Without it, every command printed
before the first `Entering directory` line lands in whatever directory
`parse-sh` itself was run in, which is the build root only by accident.

The markers are matched in English. A make running under a translated
locale announces its directories in words Bear does not match, so no
directory change is tracked at all and every entry silently carries the
starting directory. `LC_ALL=C` on the capture is what keeps both of
these right:

```sh
LC_ALL=C make -nw | bear parse-sh
```

## Ninja prints progress, not commands

`ninja -n` prints each edge's description rather than the command line
behind it:

```shell
$ ninja -n
ninja: Entering directory `build'
[1/3] Building C object CMakeFiles/demo.dir/src/a.c.o
[2/3] Building C object CMakeFiles/demo.dir/src/b.c.o
[3/3] Linking C static library libdemo.a
```

There is no command in that text, so every line is skipped and the run
exits non-zero:

```
bear: warning: line 2: skipped (glob in executable)
bear: warning: line 3: skipped (glob in executable)
bear: warning: line 4: skipped (glob in executable)
bear: warning: parse-sh: 0 command(s) parsed, 3 line(s) skipped
bear: error: Event production failed: every non-empty line was skipped; no commands parsed (see warnings above)
```

`ninja -t commands` prints the real command lines. It prints no
directory markers with them, so name the build directory with
`--directory`:

```sh
ninja -C build -t commands | bear parse-sh -C "$PWD/build"
```

Make needs no equivalent: `-n` prints a recipe line even when a leading
`@` would have silenced it in a real build, so automake's silent rules
and CMake's Makefile generator both put the full compiler command in the
dry run without `V=1` or `VERBOSE=1`.

## A log that was captured without these flags

Nothing recovers a command the text never contained, so a log missing
whole subdirectories has to be captured again, or the build run under
Bear instead. What is still fixable after the fact is the directory the
parse starts in: `--directory` sets it, for a log that came from another
machine or another checkout:

```sh
bear parse-sh -i build.log -C /home/you/project
```

[Recover compile_commands.json from a build
log](../../tutorials/compile-commands-from-a-build-log.md) walks a
handed-over log end to end.

## Read the skip report

A line using shell syntax outside the supported subset is skipped, and
each skip is reported on standard error with its line number and reason,
followed by a summary:

```shell
$ make -nw | bear parse-sh
bear: warning: line 3: skipped (shell keyword)
bear: warning: line 4: skipped (subshell)
bear: warning: parse-sh: 1 command(s) parsed, 2 line(s) skipped
```

Both lines there hid a compile: a `for` loop over sources, and a
`(cd sub && gcc ...)` subshell. A skip costs an entry only when the
skipped line was a compilation, and the run still exits 0 as long as one
line parsed, so read the summary rather than the exit code. The line
numbers are the input's, so capture to a file when you want to look them
up:

```sh
make -nw > build.log
bear parse-sh -i build.log
```

`bear parse-sh` in the [`bear(1)` man page][manpage] lists the shell
constructs the subset covers.

## When the text cannot be made complete

Every limit above is the dry run's, not the parser's, and interception
has none of them: it observes the `exec()` calls the build really makes,
so recursion, shell loops, silenced recipes, and the locale all stop
mattering.

```sh
bear -- make
```

Keep `parse-sh` for the build that cannot be run again.

## Related pages

- [Generate compile_commands.json for a Makefile
  project](compile-commands-for-makefile.md) for running the build under
  Bear instead.
- [Recover compile_commands.json from a build
  log](../../tutorials/compile-commands-from-a-build-log.md) for the
  end-to-end walkthrough of parsing a saved log.
- [Bear produces an empty compile_commands.json](empty-compilation-database.md)
  when the build did run under Bear and still produced nothing.
- [Command-line options](../../reference/command-line.md#bear-parse-sh)
  for `--input`, `--output`, `--append`, and `--directory`.
- [How Bear works](../../understanding/how-it-works.md) for what
  interception observes that text cannot carry.
- [Recipes](index.md) for the other tasks.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

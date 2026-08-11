<!-- Diataxis type: how-to -->

# Generate compile_commands.json for a Makefile project

Run this from the directory holding the Makefile:

```sh
bear -- make
```

Bear runs `make` exactly as it would run on its own, watches every
command it executes, and writes `compile_commands.json` next to it, one
entry per compiled source file. Nothing about the Makefile changes.
Make has no built-in way to export a compilation database (unlike CMake
or Meson, which do; see [the home page](../../index.md) if that is your
build system instead), so intercepting the real build is the standard
way to get one for a Make project. For the full walkthrough of a first
run, including what the output looks like, see [Getting started with
Bear](../../tutorials/getting-started.md); this page covers everything beyond that
first run.

## Autotools: ./configure and make

Run `configure` on its own, then build under Bear:

```sh
./configure
bear -- make
```

This is enough whenever preload is the active interception method:
`configure` only needs to run so it can write a Makefile, and `make` is
the command Bear needs to watch.

In wrapper mode a build step that discovers the compiler on its own has
to run under Bear too, or it never sees the wrapper. See [Configure
Bear](../../reference/configuration.md#intercept) for which mode is the
default on your platform and how to force one or the other. Practically,
the wrapper-mode requirement means running the configure step under Bear
as well, and discarding its output:

```sh
bear -- ./configure
make clean
bear -- make
```

Do not combine `./configure` and `make` into a single Bear run (for
example `bear -- sh -c './configure && make'`, or a Makefile target that
re-runs `configure` itself): `configure`'s throwaway toolchain probes
(`conftest.c` and similar) end up recorded alongside the real build. Run
`configure` and `make` as two separate Bear invocations instead, as
above; the second run's output overwrites the first's, so nothing from
the probe phase survives. See
[Troubleshooting](../troubleshooting.md#the-output-has-extra-entries) for
the mechanics, and for excluding probe files by pattern instead, if you
would rather keep one combined run.

## Incremental builds

Make only rebuilds what is out of date, and Bear only records commands
the build actually executes, so a `make` with nothing to do records
nothing: running `bear -- make` a second time in a row against an
unchanged tree overwrites `compile_commands.json` with an empty `[]`.
Build from clean when you want a complete database:

```sh
make clean
bear -- make
```

To fold in just the parts you rebuild, add `--append`. It places new
entries before the existing ones in the output file, so accumulate the
database as you touch different parts of the project instead of
rebuilding everything at once:

```sh
bear --append -- make main.o
bear --append -- make greet.o
```

After the first call the database holds one entry (`main.c`); after the
second it holds two, with the newest entry first. On the first call
there is nothing to append to, so Bear warns and ignores the option. If
a file is later rebuilt with different flags, its newest entry wins by
default; see [Configure Bear](../../reference/configuration.md#duplicates)
for the matching rule, and [Bear produces an empty
compile_commands.json](empty-compilation-database.md) if entries still
seem to go missing.

## Parallel builds

`bear -- make -j4` needs nothing special: every compiler process, however
many run concurrently, reports back to Bear on its own. A parallel build
produces the same entries as the serial one.

## Recursive Makefiles

A real build under Bear needs no extra flags for `make -C` or `$(MAKE)`
recursion: each intercepted command carries its own working directory,
so entries come out correctly scoped to the subdirectory they were
compiled in, however deep the recursion goes. In a project whose
top-level Makefile recurses into a `libgreet/` subdirectory, the
database holds `main.c` with `directory` set to the project root and
`greet.c` with `directory` set to `libgreet`, with no
`-w`/`--print-directory` needed.

The `-w` flag matters only for the dry-run path below, not for a real
build.

Bear itself, not `make`, decides where `compile_commands.json` is
written: it always lands in the directory Bear was started from, even
when the Makefile it drives is elsewhere. Running `bear -- make -C
libgreet` from inside `libgreet/` writes the file there; running the
same command from the project root writes it at the root instead.

## A build you cannot re-run

When you cannot run the build again, for example to recover a database
from a CI log after the fact, feed a dry run into `bear parse-sh`
instead of running anything:

```sh
make -n | bear parse-sh
```

For a recursive build add `-w`, so the top-level `make` also prints the
`Entering directory` markers that sub-makes already print by default;
`parse-sh` uses them to resolve each command to the right directory:

```sh
make -nw | bear parse-sh
```

Parsing a plain `make -n` for a recursive project from a directory other
than the build root puts the top-level source's `directory` at the
parsing directory rather than the build root; `make -nw` from the same
place resolves it correctly. Without `-w`, the
top-level `make` prints no directory marker of its own, so `parse-sh`
falls back to its own working directory (or `--directory`, if given) for
commands issued before any marker appears.

This path is reconstruction from text, not observation, and is lower
fidelity than a real build: a dry run can omit commands entirely
(recursive `make` does not always propagate `-n` to sub-makes, and
commands behind not-yet-generated sources never print), the environment
and `PATH` used to resolve bare executable names is the parsing one, not
the real build's, and only a documented subset of POSIX shell syntax is
understood (subshells, command substitution, and here-documents are not,
among others; an unparsed line is skipped and reported on standard error
with its line number). Prefer `bear -- make` whenever you can actually
run the build; reach for `parse-sh` only when you cannot. See `bear
parse-sh` in the [`bear(1)` man page][manpage] for the full list of what
it understands.

## Where the output goes, and how to change it

By default Bear writes `compile_commands.json` in the directory it was
started from. Use `-o`/`--output` (see [Command-line
options](../../reference/command-line.md#global-usage)) to write it
somewhere else:

```sh
bear -o build/compile_commands.json -- make
```

`-` (standard output) is not accepted for `-o` in combined mode, since
standard output belongs to the build; see the [`bear(1)`
man page][manpage] for the split `intercept`/`semantic` modes, which do
accept it.

## Related pages

- [Bear produces an empty compile_commands.json](empty-compilation-database.md)
  when even a clean build gives you nothing.
- [Set up clangd for a project without CMake](clangd-setup.md) for
  pointing your editor at the database once it exists.
- [Use Bear with ccache, distcc, or icecc](ccache-distcc-icecc.md) if
  your Makefile invokes the compiler through a launcher.
- [Run Bear inside a Docker container](docker.md) if the build itself
  runs in a container.
- [How Bear works](../../understanding/how-it-works.md) for the interception
  mechanism.
- [Command-line options](../../reference/command-line.md) and [Configure
  Bear](../../reference/configuration.md) for the flags and keys named
  above.
- [Recipes](index.md) for the other tasks.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

<!-- Diataxis type: how-to -->

# Troubleshooting

What to do when Bear runs but the result is not what you expected: an
empty or short `compile_commands.json`, entries with the wrong flags, or
a build that behaves differently under Bear than without it. If the
database is completely empty, start with [Bear produces an empty
`compile_commands.json`](recipes/empty-compilation-database.md), which
works through the causes one by one; this page collects the rest.

## Enable debug logging

Before investigating anything else, re-run the build with debug
logging on standard error:

```sh
RUST_LOG=debug bear -- <build command>
```

Without `RUST_LOG` set, Bear prints only warnings and errors. Setting it
to `debug` (or `info`/`warn`/`error` for less detail) switches every
helper process (`bear-driver`, `bear-wrapper`, the preload library) to a
verbose, timestamped format detailed enough to see which executables
were intercepted, how each was classified, and why an entry was or was
not written. Include this output when reporting a problem.

## The output is missing entries

Same causes as an empty database, but partial - a compiler ran and was
recognized, yet its entry did not survive to the file. The most common
cause is `--append` (see [Command-line
options](../reference/command-line.md#global-usage)): each Bear run
overwrites the output by default, so building incrementally without it
discards the previous run's entries. Accumulate results across runs
instead:

```sh
bear --append -- make module_a
bear --append -- make module_b
```

New entries are placed before the existing ones, so a later rebuild's
entry for a given file takes precedence over the stale one (see the
`duplicates` section below).

### A file you know is compiled twice shows up once

If a project compiles the same source file more than once - once for a
static library and once for a shared one, for example - and the
database has only one entry for it, this is not `--append`: it is the
default duplicate match dropping the second invocation. Building zlib
1.3.1 shows the effect directly. `./configure && bear -- make` produces
17 entries, one per source file, even though zlib compiles every one of
its 17 sources twice in the same directory: once for the static build,
and once with `-fPIC -DPIC` for the shared one. The two invocations
recorded for `adler32.c`:

```
gcc -O3 -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -c -o adler32.o adler32.c
gcc -O3 -fPIC -D_LARGEFILE64_SOURCE=1 -DHAVE_HIDDEN -DPIC -c -o objs/adler32.o adler32.c
```

Bear's default `duplicates.match_on` compares only `directory` and
`file` (see [Configure Bear](../reference/configuration.md#duplicates)),
and both invocations share both, so only the first is kept. Add
`arguments` to the default match, keeping `directory` and `file` in it:

```yaml
schema: "4.2"
duplicates:
  match_on:
    - directory
    - file
    - arguments
```

Rebuilding zlib with this configuration produces all 34 entries, two
per source file. Keep `directory` in the list: a project with a `util.c`
in two subdirectories, compiled with the same flags in each, would
otherwise have one of them treated as a duplicate of the other and
dropped. This is a trade-off, not a strict improvement: one
entry per file per directory is what most Clang tooling wants, and
matching on `arguments` too gives you every invocation at the cost of a
larger database in which clangd (and most other consumers) still picks
only one entry per file.

## The output has duplicate entries

Two entries are duplicates only when every field listed in
`duplicates.match_on` matches; of a set of duplicates, Bear keeps the
first one in the file - the build's first invocation within a single
run, or (per the ordering described above) the newest one across
`--append` runs.

The default match is `directory` and `file`, so one entry survives per
source file per directory whatever its arguments were. If that is
dropping invocations you wanted to keep, see [A file you know is
compiled twice shows up once](#a-file-you-know-is-compiled-twice-shows-up-once)
above for the fix and its trade-off. [Configure
Bear](../reference/configuration.md#duplicates) enumerates the fields
`match_on` accepts.

## The output has extra entries

There are two different causes, and they need different fixes.

`--append` across runs carries forward entries from earlier
invocations, including ones for files that no longer exist. Drop
`--append` and rebuild from clean to get a database with only the
current build's entries:

```sh
make clean
bear -- make -j4
```

Without `--append` each Bear run overwrites the previous output, so
`bear -- ./configure` followed by `bear -- make` leaves nothing behind
from the configure run: the build run replaces the file wholesale.

A configure step inside a *single* Bear run is the other cause, and
`--append` has nothing to do with it. `bear -- sh -c './configure &&
make'`, or a Makefile that re-runs `configure` itself, puts the
configure phase's throwaway compiles (`conftest.c` and friends) in the
same output as the real build. Split the two, so the build run's output
is the one that survives:

```sh
bear -- ./configure
make clean
bear -- make -j4
```

If the configure step has to run under Bear for the build to be
intercepted at all - wrapper mode, where the configure step must
discover the wrapper as its compiler - keep the two runs separate as
above rather than chaining them with `--append`, or exclude the probe
files with a `sources` rule.

## The build behaves differently with Bear

**Usually harmless**: extra lines in the build's own output, such as a
loader warning about `libexec.so`, or additional environment variables
Bear sets for its own bookkeeping. These do not change what gets
compiled.

**Worth reporting as a bug** if the build's actual result changes: a
different binary, a step that fails only under Bear, or output written
to an unexpected place. Two places to check first, since they are the
things Bear adds to the build environment:

- the `.bear/` wrapper directory in wrapper mode, in case the build
  system has its own use for a directory of that name in the working
  directory;
- environment variables Bear sets for interception
  (`LD_PRELOAD`/`DYLD_INSERT_LIBRARIES` in preload mode, a modified
  `PATH` in wrapper mode), in case the build inspects or forwards its
  environment in a way that is sensitive to them.

Report these with `RUST_LOG=debug` output attached.

## LD_PRELOAD errors

```
ERROR: ld.so: object '.../libexec.so' from LD_PRELOAD cannot be preloaded: ignored.
```

This means the dynamic linker could not find or load `libexec.so` at
the path `bear-driver` expects, relative to its own location
(`../$INTERCEPT_LIBDIR/libexec.so`, `lib` by default). Check that Bear
was built and installed with the same `INTERCEPT_LIBDIR` value; see
[Bear on Linux](../platforms/linux.md) for the build/install commands.
The warning does not stop the build or fail Bear: see [Bear produces an
empty compile_commands.json](recipes/empty-compilation-database.md#the-preload-library-failed-to-load)
for why this is a silent cause of a database missing exactly the entries
a subprocess would have reported.

### glibc version errors in cross-compilation

```
.../libexec.so: version `GLIBC_2.33' not found (required by .../libexec.so)
```

This is a different failure from the one above: the linker finds
`libexec.so`, but the library needs a newer glibc symbol version than
the C library available to the process it was injected into. It shows
up when the build runs compilers from a cross-compilation SDK whose
sysroot ships an older glibc than the host Bear was built on - the
preload library must be ABI-compatible with the libc the intercepted
compiler process loads from the SDK sysroot, not only with the host's.
The intercepted invocation fails outright, so that command is silently
missing from the database rather than reported as a warning.

Compare the highest glibc version Bear's preload library requires
against the highest the SDK's libc provides:

```sh
# Highest glibc version Bear's preload library requires
strings <prefix>/libexec/bear/lib/libexec.so | grep -oE 'GLIBC_[0-9.]+' | sort -V | tail -1

# Highest glibc version the SDK toolchain's libc provides
strings /path/to/sdk/sysroot/lib/libc.so.6 | grep -oE 'GLIBC_[0-9.]+' | sort -V | tail -1
```

`objdump` reports the same requirement more precisely, if it is
available:

```sh
objdump -T <prefix>/libexec/bear/lib/libexec.so | grep -oE 'GLIBC_[0-9.]+' | sort -uV | tail
```

If the version Bear's library requires is newer than what the SDK's
libc provides, build (or obtain) a Bear linked against a glibc no newer
than the SDK's, and use that build for the cross-compilation. Building
on a host whose own glibc is no newer than the SDK's avoids the
mismatch entirely.

## Getting help

1. Check `bear --version` against the [latest
   release](https://github.com/rizsotto/Bear/releases/latest);
   distribution packages are often old, see [Install
   Bear](../installation.md).
2. Run with `RUST_LOG=debug` and read the output.
3. Search [existing
   issues](https://github.com/rizsotto/Bear/issues?q=is%3Aissue).
4. Check [How Bear works](../understanding/how-it-works.md) if the
   behaviour itself is the surprise.
5. Open a new issue with the debug log and your platform (OS, Bear
   version from `bear --version`, and build system).

Related: [Command-line options](../reference/command-line.md) and
[Configure Bear](../reference/configuration.md) for the flags and keys
named above, [Bear produces an empty
compile_commands.json](recipes/empty-compilation-database.md) for that
case, the [Recipes](recipes/index.md) index for task-shaped pages, and
the platform notes for [Linux](../platforms/linux.md),
[macOS](../platforms/macos.md), [Windows](../platforms/windows.md), and
[BSD](../platforms/bsd.md).

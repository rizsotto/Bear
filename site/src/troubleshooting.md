<!-- Diataxis type: how-to -->

# Troubleshooting

What to do when Bear runs but the result is not what you expected: an
empty or short `compile_commands.json`, entries with the wrong flags, or
a build that behaves differently under Bear than without it. If the
database is completely empty, start with [Why is
`compile_commands.json` empty?](faq.md) in the FAQ; this page collects
the rest.

## Enable debug logging

Before investigating anything else, re-run the build with debug
logging on standard error:

    RUST_LOG=debug bear -- <build command>

Without `RUST_LOG` set, Bear prints only warnings and errors in a short
`bear: message` form. Setting it to `debug` (or `info`/`warn`/`error`
for less detail) switches every helper process (`bear-driver`,
`bear-wrapper`, the preload library) to a verbose format tagged with a
timestamp, level, process name and pid, and source location - detailed
enough to see which executables were intercepted, how each was
classified, and why an entry was or was not written. Include this
output when reporting a problem.

## The output is missing entries

Same causes as an empty database, but partial - a compiler ran and was
recognized, yet its entry did not survive to the file. The most common
cause is `--append`: each Bear run overwrites the output by default, so
building incrementally without it discards the previous run's entries.
Accumulate results across runs instead:

```
bear --append -- make module_a
bear --append -- make module_b
```

New entries are placed before the existing ones, so a later rebuild's
entry for a given file takes precedence over the stale one (see the
`duplicates` section below).

## The output has duplicate entries

Duplicates happen when a build compiles the same source file more than
once - a debug and a release pass, or a file rebuilt with different
flags. Bear's default duplicate rule keeps one entry per `directory`
and `file`, regardless of arguments, so the first-seen invocation wins
and later ones for the same file are dropped. To also keep separate
entries when only the arguments differ, match on `arguments` too:

```yaml
schema: "4.2"
duplicates:
  match_on:
    - file
    - arguments
```

Two entries are duplicates only when every field listed in `match_on`
matches. The [`bear(1)` man page][manpage] enumerates the fields the key
accepts.

## The output has extra entries

There are two different causes, and they need different fixes.

`--append` across runs carries forward entries from earlier
invocations, including ones for files that no longer exist. Drop
`--append` and rebuild from clean to get a database with only the
current build's entries:

```
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

```
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
[Bear on Linux](platforms/linux.md) for the build/install commands.

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

1. Run with `RUST_LOG=debug` and read the output.
2. Search [existing
   issues](https://github.com/rizsotto/Bear/issues?q=is%3Aissue).
3. Check the [FAQ](faq.md).
4. Open a new issue with the debug log and your platform (OS, Bear
   version from `bear --version`, and build system).

Related: the [FAQ](faq.md) for the empty-database case, the
[Recipes](recipes/index.md) index for task-shaped pages, and the
platform notes for [Linux](platforms/linux.md),
[macOS](platforms/macos.md), [Windows](platforms/windows.md), and
[BSD](platforms/bsd.md).

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

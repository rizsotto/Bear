<!-- Diataxis type: how-to -->

# Use Bear with ccache, distcc, or icecc

In the common case Bear needs no configuration for a launcher in front of
a compiler. Run the build exactly as you already do, launcher and all:

```
bear -- ccache gcc -c main.c -o main.o
```

produces this entry in `compile_commands.json`:

```json
[
  {
    "file": "main.c",
    "arguments": ["gcc", "-c", "main.c", "-o", "main.o"],
    "directory": "/path/to/project",
    "output": "main.o"
  }
]
```

The `ccache` token is gone. `arguments` holds the real compiler invocation,
`gcc -c main.c -o main.o`, exactly as it appeared after the launcher's own
name. distcc and icecc are dropped from the recorded command the same
way. The database never contains an entry whose command is the launcher
itself.

## What lands in `arguments`

Bear records the real compiler's argv, taken verbatim from whatever
followed the launcher's name on the command line, not a resolved or
canonicalized path:

```
bear -- ccache gcc -c main.c -o main.o           # -> "gcc", ...
bear -- ccache cc -c main.c -o main.o            # -> "cc", ...
bear -- ccache /usr/bin/gcc -c main.c -o main.o  # -> "/usr/bin/gcc", ...
```

In each case `arguments` starts with exactly the token shown in the
comment. Bear does not chase the name further; if the build wrote a bare
`cc`, that is what ends up in the database.

## The ccache masquerade case

Some distributions (Fedora, Arch, Gentoo) put a directory of symlinks
ahead of the real compilers on `PATH`, so a plain `cc` or `gcc` resolves
to ccache without the build ever typing the word `ccache`. This is
different from the case above: there is no launcher token for Bear to
drop, because the build's command line never named one. On a host where
`/usr/lib64/ccache/cc` is a symlink to `ccache`:

```
bear -- cc -c main.c -o main.o
```

records `"cc"` as the compiler, exactly as invoked, not resolved past the
symlink and not rewritten to `gcc`. Bear classifies the ambiguous `cc`
name by probing `--version` (ccache passes that probe through to the real
compiler), so the entry parses with the right flag family, but the
recorded name stays what the build actually ran. This matches the
contract in [Supported compilers](../supported-compilers.md) for
ambiguous names.

## Preload mode versus wrapper mode

Both interception modes handle an explicit launcher the same way: a
build command of `ccache gcc -c main.c -o main.o` produces the identical
single entry (compiler `gcc`, `ccache` dropped) whether `intercept.mode`
is `preload` (the Linux default) or `wrapper`.

Wrapper mode has one behavior preload mode does not need: when a build
discovers its compiler through `PATH` or a `CC` variable and that lookup
lands on a ccache masquerade directory (not an explicit `ccache` token),
Bear resolves past the masquerade at discovery time and records the real
compiler's absolute path. With `CC=gcc` and a masquerade directory first
on `PATH`, Bear logs

```
resolve: masquerade wrapper at /usr/lib64/ccache/gcc; re-resolving 'gcc' past /usr/lib64/ccache
```

and the entry records the real compiler the masquerade pointed at, never
the masquerade symlink and never Bear's own `.bear/` wrapper. See
"The recursion hazard" below for why this step exists.

## distcc specifics

distcc's own options that appear before the compiler name (`-j`,
`--jobs`, `-v`, `--verbose`, `-i`, `--show-hosts`, `--scan-avail`,
`--show-principal`) are recognized and skipped while Bear looks for the
compiler, so they never get mistaken for it and never leak into the
recorded arguments. `distcc -j4 clang -c main.c` therefore records
`clang -c main.c`, not `-j4 clang -c main.c`.

## icecc specifics

icecc follows the same contract as ccache, distcc, and sccache: the
`icecc` token is dropped and the real compiler's arguments are recorded
as invoked. icecream's `icerun` is a different tool (it runs arbitrary
commands on the cluster, not compilations) and is not recognized as a
launcher.

## The recursion hazard, and how Bear avoids it

Wrapper mode works by prepending its own directory of compiler-named
executables to `PATH`. A masquerade wrapper does the same thing for its
own purpose. If Bear recorded the masquerade wrapper's path as "the
compiler" instead of resolving past it, the two directories would look
each other up indefinitely: Bear's wrapper would call the masquerade
symlink, which would search `PATH` for the real compiler, find Bear's
own wrapper first, and call back into it. Bear avoids this by resolving
its lookup `PATH` past any masquerade directory before recording what a
bare compiler name points to, so the real compiler is what gets wrapped
and recorded, never the masquerade link or Bear's own wrapper. Detection
is a filesystem check (does the resolved path's target look like a known
launcher binary), not a subprocess probe, and it only changes what Bear
resolves, not the build's own child environment: a build that actually
wants ccache's caching behavior for other invocations still gets it.
Setting `CCACHE_COMPILER` or `CCACHE_PATH`, or stripping the masquerade
directory from the build's own `PATH`, were all considered and rejected,
because each changes the build's behaviour rather than only Bear's
lookup; the reasoning is recorded with the source, under
`docs/rationale/`.

## What does not work

- **A launcher wrapping another launcher** (`ccache distcc gcc -c
  main.c`) produces no entry. Bear does not chase a chain of launchers;
  the first launcher's argument has to be a real compiler.
- **A launcher whose argument is not a recognized compiler** (`ccache
  make all`), or a bare launcher invocation with no argument at all,
  produces no entry.
- **A launcher under a name Bear does not recognize** is not treated as
  a launcher at all: its own invocation produces no entry (it is not a
  compiler), though a real compiler it execs directly may still be
  recorded on its own. Add an override (below) to fix the launcher's own
  entry instead of relying on that side effect.

## A launcher at a nonstandard name or path

If the build wraps the compiler with a launcher under a custom path or a
renamed binary, teach Bear the mapping in the configuration file, the
same way you would hint an unrecognized compiler:

```yaml
schema: "4.2"
compilers:
  - path: /opt/build-tools/my-ccache
    as: ccache
```

With this override, `my-ccache gcc -c main.c -o main.o` records the same
single entry, compiler `gcc`, as a plain `ccache` invocation would.
`as` also accepts `distcc`, `icecc`, and `sccache` for the equivalent
case with those tools; the accepted values are listed in the `bear(1)`
man page's CONFIGURATION section. The full recognized name table (which
executables count as `gcc`, `clang`, and so on for the compiler a
launcher wraps) lives on [Supported
compilers](../supported-compilers.md); this page does not repeat it.

Related: [Supported compilers](../supported-compilers.md) for the
recognized launcher and compiler names, [Troubleshooting](../troubleshooting.md)
for a database that comes out wrong, and the [Recipes](index.md) index
for other tasks.

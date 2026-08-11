<!-- Diataxis type: tutorial -->

# Build an autotools project under Bear

By the end of this page you will have built libtasn1 4.20.0 twice: once
the ordinary way, once under Bear, and compared what each run leaves
behind. This page assumes Bear is already installed; see [Getting
started with Bear](getting-started.md) for a first run against a plain
Makefile project, or [Install Bear](../installation.md) for platform
packages.

## Get a real project

libtasn1 is a small GNU library built with automake and autoconf, a
common shape for the C and C++ projects Bear targets. Its release
tarball ships a generated `configure` script, so building it needs
nothing beyond a C compiler and `make`, no autoconf or automake
installed. Download and extract it:

```sh
curl -fsSL -O https://ftp.gnu.org/gnu/libtasn1/libtasn1-4.20.0.tar.gz
tar xzf libtasn1-4.20.0.tar.gz
cd libtasn1-4.20.0
```

GNU's release tarballs do not change after release, so this download
always matches this checksum:

```shell
$ sha256sum libtasn1-4.20.0.tar.gz
92e0e3bd4c02d4aeee76036b2ddd83f0c732ba4cda5cb71d583272b23587a76c  libtasn1-4.20.0.tar.gz
```

## Build it the ordinary way

```sh
./configure
make
```

This builds `libtasn1.so` under `lib/.libs/`, the same as it would for
anyone packaging or installing this library. Look for a compilation
database and there is none:

```shell
$ ls compile_commands.json
ls: cannot access 'compile_commands.json': No such file or directory
```

Nothing here was written to produce one; `configure` and `make` have no
notion of a compilation database. That is the gap Bear fills.

## Build it under Bear

Start from a clean tree and repeat the same two commands, this time
running `make` under Bear:

```sh
make distclean
./configure
bear -- make
```

`./configure` is exactly the command from the previous section, run the
same way, outside Bear. Only `make` runs under Bear. Preload
interception, the default on Linux, watches every process the build
spawns, so it does not matter that `configure` ran unsupervised earlier:
Bear only needs to be watching for the commands it must record, and
those are the compiler invocations `make` issues. "Build it again, in
wrapper mode" below builds this same project a second time under the
other interception method, where the configure step does need to run
under Bear.

## Look at the result

```shell
$ grep -c '"file":' compile_commands.json
34
```

libtasn1's `Makefile.am` recurses into six subdirectories, and each one
contributes its own entries: `lib` (9), `lib/gl` (3), `src` (4),
`src/gl` (14), `examples` (3), `fuzz` (1). If you get fewer, check
whether `make` stopped early: this release also builds its Texinfo
manual, which needs `makeinfo` from the `texinfo` package, and a failure
there ends the recursion before the last directories are reached. That
is worth seeing once, because it is the general case with interception:
Bear records what the build got to, so a build that stops early gives
you a database that stops with it. A recursive automake build
like this compiles in several directories at once, and gnulib's bundled
replacement sources under the two `gl` directories are compiled with
their own flags, separate from the library's.

Here is one entry from `lib`:

```json
{
  "file": "ASN1.c",
  "arguments": [
    "/usr/bin/gcc",
    "-DHAVE_CONFIG_H",
    "-I.",
    "-I..",
    "-I./gl",
    "-I./includes",
    "-DASN1_BUILDING",
    "-fanalyzer",
    "-Wall",
    "-Wextra",
    "-Wshadow",
    "-Wwrite-strings",
    "...",
    "-g",
    "-O2",
    "-c",
    "ASN1.c",
    "-fPIC",
    "-o",
    ".libs/ASN1.o"
  ],
  "directory": "/home/you/libtasn1-4.20.0/lib",
  "output": ".libs/ASN1.o"
}
```

(the real entry lists several dozen more `-W` flags; they are elided
above with `...`)

And one from `src/gl`:

```json
{
  "file": "cloexec.c",
  "arguments": [
    "/usr/bin/gcc",
    "-DHAVE_CONFIG_H",
    "-I.",
    "-I../..",
    "-Wno-cast-qual",
    "-Wno-conversion",
    "-Wno-sign-compare",
    "-Wno-unused-function",
    "-Wno-unused-parameter",
    "-g",
    "-O2",
    "-c",
    "cloexec.c",
    "-fPIC",
    "-o",
    ".libs/libsgl_la-cloexec.o"
  ],
  "directory": "/home/you/libtasn1-4.20.0/src/gl",
  "output": ".libs/libsgl_la-cloexec.o"
}
```

`lib` turns on a wall of analyzer and style warnings for libtasn1's own
code; `src/gl` turns most of them back off, because that directory holds
gnulib's imported replacement sources, not code the project wants held
to its own warning level. One database, one `directory` per entry,
describes the whole recursive build: a tool reading it gets the flags
that actually compiled each file, instead of one guessed command applied
everywhere.

## Use the database: clangd

```shell
$ clangd --check=lib/ASN1.c
I[...] Compile command from CDB is: [/home/you/libtasn1-4.20.0/lib] /usr/bin/gcc -DHAVE_CONFIG_H -I. -I.. -I./gl -I./includes -DASN1_BUILDING -fanalyzer -Wall -Wextra ... -g -O2 -c -fPIC -o .libs/ASN1.o -resource-dir=... -- /home/you/libtasn1-4.20.0/lib/ASN1.c
```

clangd pulled the same flags straight out of `compile_commands.json`,
directory and all. See [Set up clangd for a project without
CMake](../guides/recipes/clangd-setup.md) for the editor side of this hand-off.

## Build it again, in wrapper mode

Preload interception, used above, watches every process the build
spawns, so it never mattered that `./configure` ran outside Bear: the
commands Bear needed to see were `make`'s. Wrapper mode works
differently: it puts wrapper executables on `PATH` in place of the real
compilers, so a step that discovers and records the compiler has to see
the wrapper while it does so, or the build later invokes the real
compiler directly and Bear observes nothing. For libtasn1, that step is
`./configure`. See [How Bear works](../understanding/how-it-works.md) for the mechanism
behind both interception methods.

That makes libtasn1's `configure` step a case where running under Bear
is required, not just harmless. Write a `bear.yml` next to the source
tree:

```yaml
schema: "4.2"
intercept:
  mode: wrapper
```

Bear finds `bear.yml` in the current working directory on its own, so no
flag is needed; naming it explicitly with `--config bear.yml` works the
same way. This same file selects wrapper mode as the interception method
on every platform, including Linux and the BSDs, where preload is
otherwise the default. [Configure Bear](../reference/configuration.md) covers the
rest of the file's keys and the other places Bear looks for it.

From a clean tree, run `configure` and `make` as two separate Bear
invocations, in order:

```sh
make distclean
bear --config bear.yml -- ./configure
bear --config bear.yml -- make
```

The first command writes a database of its own: six entries, all of them
`configure`'s own throwaway compiler probes (`conftest.c` and similar,
plus a libtool probe under a temporary directory), not libtasn1's
sources. The second command writes the same 34 entries, across the same
six directories, as the preload build above, with no `conftest` entry
among them. Bear overwrites the output file on every run by default, so
the second command's database simply replaces the first's; the probes
are not merged in and do not need to be filtered out. Do not add
`--append` to either command here, since that flag places new entries
alongside the old ones instead of replacing them, which would let the
probes survive into the final database.

## Next steps

- [Generate compile_commands.json for a Makefile
  project](../guides/recipes/compile-commands-for-makefile.md) for the autotools
  variations this page skipped: the general `conftest.c` probe caveat,
  incremental and parallel builds.
- [Recover compile_commands.json from a build
  log](compile-commands-from-a-build-log.md) if you cannot run the build
  yourself, only read its log.
- [How Bear works](../understanding/how-it-works.md) for the interception
  mechanism.

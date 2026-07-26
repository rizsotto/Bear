<!-- Diataxis type: tutorial -->

# Getting started with Bear

By the end of this page you will have built Lua 5.4 under Bear, looked at
the `compile_commands.json` it wrote, and used that file to run clangd
and clang-tidy against the Lua source, the same way an editor or a CI
lint job would.

## Install Bear

Install Bear with your platform's package manager; see
[Install Bear](../installation.md) for the commands. Then confirm it is on
your `PATH`:

```sh
bear --version
```

## Get a real project

Lua 5.4 is a good first target: a plain Makefile, no configure step, no
dependencies beyond a C compiler and `make`. Download and extract it:

```sh
curl -fsSL -O https://www.lua.org/ftp/lua-5.4.8.tar.gz
tar xzf lua-5.4.8.tar.gz
cd lua-5.4.8
```

`lua.org`'s ftp directory does not change existing releases, so this
download always matches this checksum:

```shell
$ sha256sum lua-5.4.8.tar.gz
4f18ddae154e793e46eeab727c59ef1c0c0c2b744e7b94219710d76f530629ae  lua-5.4.8.tar.gz
```

## Build it under Bear

```sh
bear -- make
```

Bear runs the build exactly as `make` would on its own, forwarding its
output, and adds one thing: a `compile_commands.json` file in the
current directory, one entry per compiled source file. Lua's Makefile
recurses into `src`, so this produces 34 entries, one for each `.c` file
that went into `lua` and `luac`.

## Look at the result

```sh
cat compile_commands.json
```

Each entry looks like this:

```json
{
  "file": "lapi.c",
  "arguments": [
    "/usr/bin/gcc",
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
  "directory": "/home/you/lua-5.4.8/src",
  "output": "lapi.o"
}
```

`directory` is where the compiler ran, `file` is the source it compiled
(relative to `directory`), `arguments` is the exact command as an array,
and `output` is the object file it produced. Your own output will show
your own resolved compiler path in `arguments[0]`; on a distribution
that routes compilers through `ccache`, that path may be a ccache
symlink instead of the compiler itself. There is one entry per compiled
source and none for the headers Lua's sources include, such as
`lapi.h`, because Bear only records commands the build actually ran,
and the build never invokes the compiler on a header directly. For what
Bear does and does not capture, and why, see
[How Bear works](../understanding/how-it-works.md).

## Use the database: clangd

`compile_commands.json` exists for tools like clangd to read, not to be
read by people. Point clangd at one Lua source file and it logs the
exact command it pulled from the database:

```shell
$ clangd --check=src/lapi.c
I[...] Compile command from CDB is: [/home/you/lua-5.4.8/src] /usr/bin/gcc -std=gnu99 -O2 -Wall -Wextra -DLUA_COMPAT_5_3 -DLUA_USE_LINUX -c -o lapi.o -resource-dir=... -- /home/you/lua-5.4.8/src/lapi.c
```

That is the whole hand-off: an editor with clangd support opened on
this directory gets working "go to definition", completion, and
diagnostics with no other setup. See [Set up clangd for a project
without CMake](../guides/recipes/clangd-setup.md) for the editor side.

## Use the database: clang-tidy

clang-tidy reads the same file to know how to parse each source it
lints. Run it against one Lua source file, restricted to a single check
so the output stays short:

```shell
$ clang-tidy -p . -checks='-*,clang-analyzer-deadcode.DeadStores' src/lfunc.c
1 warning generated.
lfunc.c:196:42: warning: Although the value stored to 'upl' is used in the enclosing expression, the value is never actually read from 'upl' [clang-analyzer-deadcode.DeadStores]
  196 |   while ((uv = L->openupval) != NULL && (upl = uplevel(uv)) >= level) {
      |                                          ^
```

That is a real finding in Lua's own source, produced by clang-tidy, not
by Bear: Bear's job ends at writing `compile_commands.json`, and every
tool built on Clang's tooling API, from editors to linters to
refactoring tools, takes it from there.

## Next steps

- [Generate compile_commands.json for a Makefile
  project](../guides/recipes/compile-commands-for-makefile.md) for variations on
  what you just did: autotools, incremental and parallel builds,
  recursive Makefiles.
- [Set up clangd for a project without
  CMake](../guides/recipes/clangd-setup.md) if the editor side needs more than the
  default lookup.
- [Recover compile_commands.json from a build
  log](compile-commands-from-a-build-log.md) if you cannot run the build
  yourself, only read its log.
- [Bear produces an empty
  `compile_commands.json`](../guides/recipes/empty-compilation-database.md) if a
  real project's database comes out empty, and
  [Troubleshooting](../guides/troubleshooting.md) if it comes out wrong.
- [How Bear works](../understanding/how-it-works.md) for the interception mechanism
  behind this page.

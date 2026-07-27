<!-- Diataxis type: how-to -->

# Generate compile_commands.json for an Emscripten project

Run the build under Bear, with Bear as the outermost command:

```sh
bear -- emmake make
```

Bear captures every process the build tree starts, however many wrapper
scripts sit in between, so it does not matter that `emmake` itself sets
`CC`/`CXX` to `emcc`/`em++` before handing off to `make`: those exec calls
happen inside the tree Bear is already watching. Bear has to be the
outermost command for that to hold, so `bear -- emmake make` is the order
to use.

`emcc`, `em++`, and their `.py`-suffixed spellings are recognized under
the `clang` family, not a separate `emscripten` id, since Emscripten's
driver is a Clang wrapper whose command line follows Clang's flag syntax
closely enough for Bear's Clang rules to parse it correctly. There is no
`emscripten` `as` value to write in a `compilers:` override; use `clang`
if a nonstandard `emcc` path needs a hint. See [Supported
compilers](../../reference/supported-compilers.md) for the full
recognized-name table.

## CMake projects: emcmake

For a CMake-based Emscripten project, `emcmake` only needs to run once, to
generate a build tree configured for `emcc`/`em++`; the actual build is
what Bear needs to watch:

```sh
emcmake cmake -B build
bear -- cmake --build build
```

This is the same two-step shape as the general CMake case in the
[`bear(1)` man page][manpage]'s EXAMPLES: the generation step does not
compile your project's sources, so it does not need to run under Bear.
In wrapper mode (the default on macOS and Windows, or forced with
`intercept.mode: wrapper`), a step that discovers the compiler on its own
has to run under Bear too, so run the generation step under Bear as well
and discard its output, the same way [Generate compile_commands.json for
a Makefile project](compile-commands-for-makefile.md#autotools-configure-and-make)
handles `./configure`:

```sh
bear -- emcmake cmake -B build
bear -- cmake --build build
```

## clangd and an `emcc` entry

clangd reads the recorded compiler as `emcc`, a binary it does not run
and does not know the way it knows a plain `clang`. When it cannot infer
a driver's built-in include paths and default target from the recorded
command alone, point it at the real driver directly with
`--query-driver`, the same mechanism the [`bear(1)` man
page][manpage] documents for MPI compiler wrappers:

```sh
clangd --query-driver=/path/to/emcc
```

See [Set up clangd for a project without CMake](clangd-setup.md) for
getting the database itself into a place clangd finds it.

## Related pages

- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table and the `compilers:` override for a path Bear
  gets wrong.
- [Generate compile_commands.json for a Makefile
  project](compile-commands-for-makefile.md) for the Autotools-style
  two-step pattern this page reuses for `emcmake`.
- [Set up clangd for a project without CMake](clangd-setup.md) for
  pointing your editor at the database this page produces.
- [Recipes](index.md) for the rest of the task pages.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

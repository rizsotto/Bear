<!-- Diataxis type: how-to -->

# Set up clangd for a project without CMake

```
bear -- make
```

Run your build under Bear once to produce `compile_commands.json`, and
clangd picks it up automatically the next time you open a source file in
your editor - no CMake, no manual flag lists. The rest of this page is
about getting that file where clangd looks for it, and confirming that
it actually found it.

## Where the file has to be

clangd searches for `compile_commands.json` starting from the file you
are editing: the file's own directory, a `build/` subdirectory of it,
then the parent directory, that parent's `build/` subdirectory, and so
on up the tree (see clangd's [project setup
documentation][clangd-install]). Running `bear -- make` from your
project root places the file exactly where this search finds it for any
source file under that root, which is why the plain invocation above is
usually all you need.

Two situations fall outside that search:

- the database was generated somewhere other than the project root, for
  example a separate build directory Bear was pointed at with `bear
  --output build/compile_commands.json` or `bear intercept`'s own
  `--output`;
- you are editing a source tree that clangd's search does not reach from
  the file's own location.

## Pointing clangd at a database in a different place

Pass `--compile-commands-dir` on clangd's own command line (consult your
editor's clangd-integration settings for where to add this):

```
clangd --compile-commands-dir=/path/to/build
```

or, to keep it in the project instead of your editor configuration, add
a `.clangd` file at the project root:

```yaml
CompileFlags:
  CompilationDatabase: build
```

The `CompilationDatabase` key takes a directory (absolute, or relative
to the `.clangd` file's own location) to search instead of walking
upward from the edited file. See clangd's [configuration
documentation][clangd-config] for the complete set of `.clangd` keys;
this page only covers the one that matters for finding the database.

With `compile_commands.json` in a `build/` directory
and sources in a sibling `src/` directory, both `clangd
--compile-commands-dir=build` and a `.clangd` file with
`CompilationDatabase: build` at the project root make clangd load the
database for a file under `src/`.

## Confirming clangd actually loaded it

Ask clangd to parse a single file outside of your editor:

```
clangd --check=path/to/file.c
```

Look at the lines it logs while loading:

```
Loaded compilation database from /path/to/build/compile_commands.json
Compile command from CDB is: [...] /usr/bin/cc -o main ...
```

`Compile command from CDB is:` means clangd found and used an entry for
that exact file. There are two other outcomes worth telling apart.

```
Compile command inferred from greet.c is: [...] -x c-header ... -- /path/to/greet.h
```

clangd found no entry for this file but borrowed one from a neighbour in
the same directory. This is the normal, working result for a header,
since Bear records the sources a build compiles and headers are not
compiled on their own. Take it as success unless the flags it inferred
are wrong for that file.

```
Generic fallback command is: [...] clang ... -- /path/to/file.c
```

clangd loaded *some* database but found nothing to work from, not even a
neighbour - a sign the file was filtered out of `compile_commands.json`,
or was never compiled (see [Bear produces an empty
compile_commands.json](empty-compilation-database.md) if the database
itself is empty). If it never mentions loading a database at all, it
did not find one on its search path or at the configured location; see
[Where the file has to be](#where-the-file-has-to-be) above.

Related: [Bear produces an empty
compile_commands.json](empty-compilation-database.md) if the file has
no entries at all, [Generate compile_commands.json for a Makefile
project](compile-commands-for-makefile.md), and the
[Recipes](index.md) index.

  [clangd-install]: https://clangd.llvm.org/installation.html
  [clangd-config]: https://clangd.llvm.org/config.html

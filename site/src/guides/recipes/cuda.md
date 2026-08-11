<!-- Diataxis type: how-to -->

# Generate compile_commands.json for a CUDA project

Run the build under Bear exactly as you already run it:

```sh
bear -- make
```

Bear recognizes `nvcc`, NVIDIA's CUDA compiler driver, under the `cuda`
family (`as: cuda`), including a cross-compilation target prefix and a
version suffix (`aarch64-linux-gnu-nvcc`, `nvcc-12.0`). The recorded entry
looks like any other compiler entry, with `nvcc`-specific flags
(`--gpu-architecture`, `-gencode`, `-Xcompiler`, and similar) parsed and
kept in `arguments` verbatim:

```json
[
  {
    "file": "main.cu",
    "arguments": ["nvcc", "-c", "main.cu", "-o", "main.o"],
    "directory": "/path/to/project",
    "output": "main.o"
  }
]
```

See [Supported compilers](../../reference/supported-compilers.md) for the full
recognized-name table and how prefix/suffix recognition works in general.

## clangd and clang-tidy do not speak the same nvcc dialect

`nvcc`'s command-line flags are its own, not Clang's CUDA mode: options
such as `-gencode`, `--generate-code`, `-Xcompiler`, `-Xptxas`, and
`--extended-lambda` have no equivalent that Clang-based tooling accepts,
so an entry recorded straight from an `nvcc` invocation can make clangd
or clang-tidy reject the command line outright when they try to parse a
`.cu` file with it. Bear itself has no option to filter or rewrite
individual flags in a recorded entry; `format.arguments` in the
[`bear(1)` man page][manpage] only covers `@file` expansion and a couple
of environment-variable foldings, and `sources` only decides which files
get an entry, not what their recorded arguments contain.

The fix is on the consumer side. A `.clangd` file can strip the
unsupported flags before clangd parses the entry:

```yaml
CompileFlags:
  Remove: [-gencode*, --generate-code*, -Xcompiler*, -Xptxas*, --extended-lambda]
```

If you would rather not have clangd touch `.cu` files at all, suppress
its diagnostics on them the same way [the `bear(1)` man
page][manpage] recommends for the `valac` entries of a mixed C/Vala
project:

```yaml
If:
  PathMatch: .*\.cu
Diagnostics:
  Suppress: '*'
```

Or drop CUDA sources from the database entirely with a `sources` rule in
`bear.yml` (see CONFIGURATION in the man page):

```yaml
sources:
  files:
    - pattern: "*.cu"
      action: exclude
```

## nvcc's host-compiler sub-invocations

`nvcc` splits a `.cu` file into a device-code path and a host-code path,
and for the host side it generates an intermediate file and compiles that
with the host compiler (`gcc`, `clang`, or MSVC's `cl`, depending on
platform) as a separate process. Bear intercepts that process the same
way it intercepts `nvcc` itself, and because the host compiler's
intermediate file has a different name than the original `.cu` source,
this sub-invocation does not collide with `nvcc`'s own entry under Bear's
default duplicate matching (`directory` and `file`): both land in
`compile_commands.json` as separate entries, one for the `.cu` file and
one for the generated intermediate file the host compiler actually saw.
The intermediate file's entry is rarely useful to an editor, since the
file it names does not exist once the build finishes; exclude it the same
way as `.cu` files above, with a pattern matching the intermediate
filename your local `nvcc` generates (for example `"*.cudafe1.cpp"`), or
scope a `sources` rule to `directories` instead if the host-compiler pass
runs from its own temporary directory.

## Related pages

- [Supported compilers](../../reference/supported-compilers.md) for the full
  recognized-name table and the `compilers:` override for a path Bear
  gets wrong.
- [Generate compile_commands.json for a Makefile
  project](compile-commands-for-makefile.md) for the general Make
  workflow.
- [Set up clangd for a project without CMake](clangd-setup.md) for
  pointing your editor at the database once it exists.
- [Recipes](index.md) for the other tasks.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

<!-- Diataxis type: how-to -->

# Generate compile_commands.json for a CMake project

CMake writes `compile_commands.json` itself; turn that on before reaching
for Bear:

```sh
cmake -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
cmake --build build
```

CMake places the file in the build directory, the one passed to `-B`
(`build` above), not the source tree. Setting the same variable through a
`CMakePresets.json` `cacheVariables` entry, instead of the `-D` flag on
the command line, has the same effect. Point your editor at the file
this produces with [Set up clangd for a project without
CMake](clangd-setup.md), which covers `--compile-commands-dir` and the
`.clangd` `CompilationDatabase` key.

This is the better answer for the ordinary CMake project, and the rest of
this page is about the cases where it is not enough.

## Generators that ignore the variable

CMake's own documentation for `CMAKE_EXPORT_COMPILE_COMMANDS` states:
"This option is implemented only by Makefile Generators and Ninja
Generators. It is ignored on other generators." The Visual Studio and
Xcode generators fall into "other generators": setting the variable with
either of those has no effect, and no `compile_commands.json` appears no
matter how the project builds. Reach for Bear instead, pointed at
whatever actually invokes the compiler (see [Command-line
options](../../reference/command-line.md) for the `--` split between
Bear's own flags and the build command):

```sh
bear -- cmake --build build
```

This works regardless of which generator produced the build files; see
[How Bear works](../../understanding/how-it-works.md) for why watching
the build succeeds where the generator's own export does not.

## When the export does not match what actually ran

CMake's export records the command it intends to run, generated at
configure time. Three situations make that different from what the build
actually executes:

- a compiler launcher in front of the compiler
  (`CMAKE_C_COMPILER_LAUNCHER`, `CMAKE_CXX_COMPILER_LAUNCHER`, or a
  `ccache`/`distcc` setup baked into the toolchain file);
- a wrapper script standing in for the compiler;
- a custom command or custom target that compiles something outside
  CMake's own compile rules.

Interception records what the build actually ran instead of what CMake
planned:

```sh
bear -- cmake --build build
```

See [Use Bear with ccache, distcc, or icecc](ccache-distcc-icecc.md) for
the launcher case in detail.

## A superbuild, or CMake as one step of a larger build

When CMake is only part of the build - a superbuild that configures and
builds several subprojects, or a top-level Makefile that drives `cmake`
alongside other steps - CMake's own export only ever covers the CMake
part. Run the whole thing under Bear instead, and every compiler
invocation ends up in the same database regardless of which step
produced it:

```sh
bear -- make
```

or, when the top-level driver is itself CMake:

```sh
bear -- cmake --build .
```

## If the project does not use CMake

CMake is not the only build system that writes `compile_commands.json`
itself. Check whether yours already does before reaching for Bear:

- Ninja: `ninja -t compdb > compile_commands.json`
- Qbs: `qbs generate --generator clangdb`
- Waf: load the `clang_compilation_database` tool from `wscript`, then
  run `waf clangdb`
- Bazel: no native flag, but [Hedron's Compile Commands
  Extractor](https://github.com/hedronvision/bazel-compile-commands-extractor)
  does the same job as a third-party target
- Clang itself: pass `-MJ <fragment>.json` to every invocation and
  concatenate the fragments into a single JSON array

Use your build system's own export when it has one. Reach for Bear when
it does not, or when what it exports does not match what actually ran.

## clangd cannot find the file CMake wrote

`compile_commands.json` living in the build directory rather than the
project root is the usual reason clangd does not pick it up
automatically. See [Set up clangd for a project without
CMake](clangd-setup.md) for how clangd searches and for pointing it at
the exact directory, and [Bear produces an empty
compile_commands.json](empty-compilation-database.md) if the file exists
but is missing entries rather than missing entirely.

Related: [Generate compile_commands.json for a Makefile
project](compile-commands-for-makefile.md) for the equivalent Bear-only
workflow when there is no CMake export to fall back on, and the
[Recipes](index.md) index.

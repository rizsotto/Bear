<!-- Diataxis type: landing (navigation page, not one of the four types) -->

# Generate compile_commands.json for any C or C++ build

Bear gives you working code navigation, autocomplete, and diagnostics on
any C or C++ project, whatever the build system. Prefix your build
command with `bear --` and Bear produces the `compile_commands.json`
file that Clang tooling (clangd, clang-tidy, and friends) needs to
understand your code:

```sh
bear -- make
```

Bear captures the compiler invocations while your build runs and writes
the [JSON compilation database][jsoncdb] for you. No changes to your
build system are required.

  [jsoncdb]: https://clang.llvm.org/docs/JSONCompilationDatabase.html

## When to use Bear

Bear is the tool of choice when the build system cannot produce a
compilation database for you:

- Make, autotools, or custom script builds.
- A build you cannot or do not want to modify: a third-party project, a
  codebase you are just exploring, a CI job.
- Unusual toolchains: embedded, HPC, CUDA, cross-compilers, and compiler
  launchers such as ccache, distcc, and icecc.
- A build you cannot run at all: Bear can read `make -n` dry-run output
  or a saved build log instead of running the build.

If your project uses CMake or Meson, those tools can export a
compilation database directly, and Bazel has it through Hedron's
[bazel-compile-commands-extractor][hedron]. Use that when it is
available; [Generate compile_commands.json for a CMake
project](guides/recipes/cmake.md) covers those exports and the cases where they
are not enough.

  [hedron]: https://github.com/hedronvision/bazel-compile-commands-extractor

## Where to go next

- [Getting started with Bear](tutorials/getting-started.md) - the first successful
  run, start to finish.
- [Install Bear](installation.md) - packages and building from source.
- [Recipes](guides/recipes/index.md) - one page per task.
- [Troubleshooting](guides/troubleshooting.md) - when the database is empty or
  incomplete.
- [How Bear works](understanding/how-it-works.md) - the mechanism behind the results.

The complete reference for command-line options, configuration keys, and
their defaults is the [`bear(1)` man page][manpage], not this site.

  [manpage]: https://github.com/rizsotto/Bear/blob/master/man/bear.1.md

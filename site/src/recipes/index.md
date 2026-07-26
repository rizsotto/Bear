<!-- Diataxis type: landing (navigation page, not one of the four types) -->

# Recipes

One page per task. Each recipe answers a single question and opens with
the command that answers it.

- [Generate compile_commands.json for a Makefile project](compile-commands-for-makefile.md)
  - the main recipe: plain Make, autotools, incremental and parallel
  builds, recursive Makefiles, and recovering a database without
  running the build.
- [Bear produces an empty compile_commands.json](empty-compilation-database.md)
  - what to check when the file comes out empty or short.
- [Set up clangd for a project without CMake](clangd-setup.md) - point
  your editor at the database once Bear has written it.
- [Use Bear with ccache, distcc, or icecc](ccache-distcc-icecc.md) -
  keep a compiler launcher in the build while Bear still records the
  real compiler invocation.
- [Run Bear inside a Docker container](docker.md) - capture a build that
  runs inside a container.

Related pages: [Getting started with Bear](../getting-started.md) for
the first run, [Troubleshooting](../troubleshooting.md) for a database
that came out wrong, and [How Bear works](../how-it-works.md) for the
mechanism.

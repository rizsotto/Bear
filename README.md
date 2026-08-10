[![Packaging status](https://repology.org/badge/tiny-repos/bear-clang.svg)](https://repology.org/project/bear-clang/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/rust%20CI/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)

ʕ·ᴥ·ʔ Build EAR
===============

Bear gives you working code navigation, autocomplete, and diagnostics on
any C/C++ project, whatever the build system. Prefix your build command
with `bear --` and it produces the `compile_commands.json` file that
Clang tooling (clangd, clang-tidy, and friends) needs to understand your
code.

The [JSON compilation database][JSONCDB] describes how each translation unit
is compiled. Bear captures the compiler invocations while your build runs
and writes the database for you - no changes to your build system required.

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html

When to use Bear
-----------------

Bear is the tool of choice when the build system cannot produce a
compilation database for you:

- Make, autotools, or custom script builds - most of the C world.
- A build you cannot or do not want to modify: a third-party project,
  a codebase you are just exploring, a CI job.
- Unusual toolchains: embedded (ARM, TI, Microchip, QNX), HPC (Cray,
  Intel, NVIDIA, MPI wrappers), CUDA, cross-compilers, and compiler
  launchers such as ccache, distcc, and icecc.
- A build you cannot run at all: parse `make -n` dry-run output or a
  saved build log instead.

Bear can intercept the build as it runs, recording what the compiler was
actually invoked with, including flags injected by wrappers and entries for
generated sources. Bear can also interpret the commands a build system plans
to run, formatting those into a compilation database as if they had executed.

If your project uses CMake, Meson, or Bazel, those tools can export a
compilation database directly; use that when it is available.

How to install
--------------

Bear is [packaged][REPOLOGY] for many distributions. Check your
distribution's package manager first. Alternatively, you can
[build it](INSTALL.md) from source.

How to use
----------

After installation, run:

    bear -- <your-build-command>

Bear writes `compile_commands.json` to the current working directory. Pass
Bear's own options before `--`; everything after that is treated as part of
the build command.

If you cannot run the build - all you have is a CI log or the build system's
dry-run output - Bear can reconstruct the database from that text instead:

    make -n | bear parse-sh

For more, run `bear --help` or read the man page. The
[documentation site][DOCS] covers limitations, known issues, and
platform- and toolchain-specific usage.

Limitations
-----------

Bear works on Linux, macOS, FreeBSD, OpenBSD, NetBSD, DragonFly BSD,
and Windows.

Each operating system and build system imposes constraints on how Bear can
run and which interception method is available, and those affect the final
output. The [documentation site][DOCS] describes the available options and
each option's limitations.

Acknowledgments
---------------

Bear is kept going by people who chip in. Thank you.

- [@maflcko](https://github.com/maflcko) for regular sponsorship.
- [Anthropic](https://claude.ai/) for a free Claude subscription, which
  supported development work.

Problem reports
---------------

Before opening a new problem report, please check the documentation to see if
your problem is a known issue with a documented workaround. It's also helpful
to look at older (possibly closed) [issues][ISSUES] before opening a new one.

If you decide to report a problem, please provide as much context as possible
to help reproduce the error. If you just have a question about usage, please
don't be shy; ask your question in an issue or in our [chat][CHAT].

If you've found a bug and have a fix for it, please share it by opening a pull
request.

Please follow the [contribution guide][GUIDE] when you do.

  [ISSUES]: https://github.com/rizsotto/Bear/issues
  [DOCS]: https://rizsotto.github.io/Bear/
  [CHAT]: https://gitter.im/rizsotto/Bear/discussions
  [GUIDE]: https://github.com/rizsotto/Bear/blob/master/CONTRIBUTING.md
  [REPOLOGY]: https://repology.org/project/bear-clang/versions
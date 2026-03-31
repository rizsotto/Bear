[![Packaging status](https://repology.org/badge/tiny-repos/bear-clang.svg)](https://repology.org/project/bear-clang/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/rust%20CI/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)
[![Gitter](https://img.shields.io/gitter/room/rizsotto/Bear)](https://gitter.im/rizsotto/Bear)

ʕ·ᴥ·ʔ Build EAR
===============

Bear generates a compilation database for Clang tooling.

The [JSON compilation database][JSONCDB] describes how each translation unit
is compiled. Clang-based tools use it to understand compiler flags, include
paths, and other build settings.

Some build systems can generate a JSON compilation database directly. For
build systems that cannot, Bear captures compiler invocations during the
build and writes the database for you.

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html

How to install
--------------

Bear is [packaged](https://repology.org/project/bear-clang/versions) for many
distributions. Check your distribution's package manager first. Alternatively,
you can [build it](INSTALL.md) from source.

How to use
----------

After installation, run:

    bear -- <your-build-command>

Bear writes `compile_commands.json` to the current working directory.

For more options, see the man page or run `bear --help`. Pass Bear’s own
options before `--`; everything after that is treated as part of the build command.

Please be aware that some package managers still ship the 2.4.x release. In
that case, please omit the extra `--` or consult your local documentation.

For more information, read the man pages or the project [wiki][WIKI], which
talks about limitations, known issues, and platform-specific usage.

When to use Bear
-----------------

Use Bear when your build system does not natively support generating a
[JSON compilation database][JSONCDB]. If your project already uses CMake, Meson,
or Bazel, prefer the built-in compilation database export those tools provide;
it is usually faster and more reliable.

Supported platforms
-------------------

Bear works on Linux, macOS, FreeBSD, OpenBSD, NetBSD, DragonFly BSD,
and Windows.

Limitations
-----------

Bear works by intercepting compiler calls during a build. This means certain
environments may need extra configuration — for example, macOS System
Integrity Protection (SIP) or sandboxed builds (Nix, Flatpak). See the
[wiki][WIKI] for details and workarounds.

Problem reports
---------------

Before opening a new problem report, please check the [wiki][WIKI] to see if
your problem is a known issue with a documented workaround. It's also helpful
to look at older (possibly closed) [issues][ISSUES] before opening a new one.

If you decide to report a problem, please provide as much context as possible
to help reproduce the error. If you just have a question about usage, please
don't be shy; ask your question in an issue or in our [chat][CHAT].

If you've found a bug and have a fix for it, please share it by opening a pull
request.

Please follow the [contribution guide][GUIDE] when you do.

  [ISSUES]: https://github.com/rizsotto/Bear/issues
  [WIKI]: https://github.com/rizsotto/Bear/wiki
  [CHAT]: https://gitter.im/rizsotto/Bear/discussions
  [GUIDE]: https://github.com/rizsotto/Bear/blob/master/CONTRIBUTING.md
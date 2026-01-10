[![Packaging status](https://repology.org/badge/tiny-repos/bear-clang.svg)](https://repology.org/project/bear-clang/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/rust%20CI/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)
[![Gitter](https://img.shields.io/gitter/room/rizsotto/Bear)](https://gitter.im/rizsotto/Bear)

ʕ·ᴥ·ʔ Build EAR
===============

Bear is a tool that generates a compilation database for clang tooling.

The [JSON compilation database][JSONCDB] is used in the clang project to
provide information on how a single compilation unit is processed. With this,
it is easy to re-run the compilation with alternate programs.

Some build systems natively support the generation of a JSON compilation
database. For projects that do not use such build tools, Bear generates the
JSON file during the build process.

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html

How to install
--------------

Bear is [packaged](https://repology.org/project/bear-clang/versions) for many
distributions. Check your distribution's package manager. Alternatively, you
can [build it](INSTALL.md) from source.

How to use
----------

After installation, use it like this:

    bear -- <your-build-command>

The output file, `compile_commands.json`, is saved in the current directory.

For more options, you can check the man page or pass the `--help` parameter.
Note that if you want to pass parameters to Bear, pass them _before_ the `--`;
everything after that is considered part of the build command.

Please be aware that some package managers still ship the 2.4.x release. In
that case, please omit the extra `--` or consult your local documentation.

For more information, read the man pages or the project [wiki][WIKI], which
talks about limitations, known issues, and platform-specific usage.

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
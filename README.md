[![Packaging status](https://repology.org/badge/tiny-repos/bear-clang.svg)](https://repology.org/project/bear-clang/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/continuous%20integration/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)
[![Gitter](https://img.shields.io/gitter/room/rizsotto/Bear)](https://gitter.im/rizsotto/Bear)

ʕ·ᴥ·ʔ Build EAR
===============

Bear is a tool that generates a compilation database for clang tooling.

The [JSON compilation database][JSONCDB] is used in the clang project to
provide information on how a single compilation unit is processed. With this,
it is easy to re-run the compilation with alternate programs.

<<<<<<< HEAD
Some build systems natively support the generation of a JSON compilation
database. For projects that do not use such a build tool, Bear generates the
JSON file during the build process.
=======
Some build systems natively support the generation of JSON compilation
database. For projects that do not use such build tools, Bear generates
the JSON file during the build process.
>>>>>>> 4.0-rc

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html

How to install
--------------

Bear is [packaged](https://repology.org/project/bear-clang/versions) for many
distributions. Check out your package manager. Or [build it](INSTALL.md)
from source.

How to use
----------

After installation, use Bear like this:

    bear -- <your-build-command>

The output file, `compile_commands.json`, is saved in the current directory.

<<<<<<< HEAD
For more options, check the man page or pass the `--help` parameter. Note that
if you want to pass parameters to Bear, pass those _before_ the `--` sign,
everything after that will be the build command.

Please be aware that some package managers still ship our old 2.4.x release. In
that case, please omit the extra `--` sign or consult your local documentation.
=======
For more options, you can check the man page or pass the `--help` parameter. Note
that if you want to pass parameters to Bear, pass those _before_ the `--` sign;
everything after that will be the build command.

Please be aware that some package managers still ship our old 2.4.x release.
In that case please omit the extra `--` sign or consult your local documentation.
>>>>>>> 4.0-rc

For more, read the man pages or [wiki][WIKI] of the project, which talks about
limitations, known issues and platform-specific usage.

Problem reports
---------------

Before you open a new problem report, please look at the [wiki][WIKI] if your
problem is a known one with a documented workaround. It's also helpful to look
at older (maybe closed) [issues][ISSUES] before you open a new one.

If you decide to report a problem, try to give as much context as possible to
help me reproduce the error you see. If you have a question about usage, please
don't be shy, ask your question in an issue or in [chat][CHAT].

If you found a bug, but also found a fix for it, please share it with me and
open a pull request.

Please follow the [contribution guide][GUIDE] when you do these.

  [ISSUES]: https://github.com/rizsotto/Bear/issues
  [WIKI]: https://github.com/rizsotto/Bear/wiki
  [CHAT]: https://gitter.im/rizsotto/Bear
  [GUIDE]: https://github.com/rizsotto/Bear/blob/master/CONTRIBUTING.md

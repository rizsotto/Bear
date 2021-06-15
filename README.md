[![Packaging status](https://repology.org/badge/tiny-repos/bear.svg)](https://repology.org/project/bear/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/continuous%20integration/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)
[![Gitter](https://img.shields.io/gitter/room/rizsotto/Bear)](https://gitter.im/rizsotto/Bear)

ʕ·ᴥ·ʔ Build EAR  
===============

Bear is a tool that generates a compilation database for clang tooling.

The [JSON compilation database][JSONCDB] is used in the clang project
to provide information on how a single compilation unit is processed.
With this, it is easy to re-run the compilation with alternate programs.

One way to get a compilation database is to use `cmake` as the build
tool. Passing `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON` to cmake generates
the `compile_commands.json` file into the current directory.

For non-cmake projects, Bear generates the JSON file during the build process.

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html

How it works
------------

The concept behind Bear is: to execute the original build command and
intercept the command executions issued by the build tool. From the
log of command executions it tries to identify the compiler calls and
creates the final compilation database.  
 
For intercepting the compiler executions, Bear uses the `LD_PRELOAD`
or `DYLD_INSERT_LIBRARIES` mechanisms provided by the dynamic linker.
When the dynamic linker is not working (because the executable is not a
dynamically linked executable or security protection disables the linker)
then Bear uses compiler wrappers to record the compiler calls. The wrapper
sends execution report and calls the real compiler. (Not only compilers,
but linkers, assemblers and other tools are also wrapped.)

The intercepted commands are filtered and only the compiler executions
goes into the compilation database. Those commands which ends up in the
final output are transformed a little bit (some compiler flags are
removed). 

How to install
--------------

Bear is [packaged](https://repology.org/project/bear/versions) for many distributions. Check out your package manager.
Or [build it](INSTALL.md) from source.

How to use
----------

After installation the usage is like this:

    bear -- <your-build-command>

The output file called `compile_commands.json` is saved in the current directory.

For more options you can check the man page or pass `--help` parameter. Note
that if you want to pass parameter to Bear, pass those _before_ the `--` sign,
everything after that will be the build command. 

Side note: Since Bear is executing the build command, only those commands will
be recorded which were actually executed during the current build. Which means
if you have already built your project and you re-run the build command with
Bear you probably end up to have an empty output. (Practically it means you
need to run `make clean` before you run `bear -- make`.)

For more read the [wiki][WIKI] of the project, which talks about limitations,
known issues and platform specific usage. 

Problem reports
---------------

Before you open a new problem report, please look at the [wiki][WIKI] if your
problem is a known one with documented workaround. It's also helpful to look
at older (maybe closed) [issues][ISSUES] before you open a new one.  

If you decided to report a problem, try to give as much context as it would
help me to reproduce the error you see. If you just have a question about the
usage, please don't be shy, ask your question in an issue or in [chat][CHAT].

If you found a bug, but also found a fix for it, please share it with me and
open a pull request.

Please follow the [contribution guide][GUIDE] when you do these.

  [ISSUES]: https://github.com/rizsotto/Bear/issues
  [WIKI]: https://github.com/rizsotto/Bear/wiki
  [CHAT]: https://gitter.im/rizsotto/Bear
  [GUIDE]: https://github.com/rizsotto/Bear/blob/master/CONTRIBUTING.md

---

Thanks to [JetBrains](https://www.jetbrains.com/?from=Bear)
for donating product licenses to help develop Bear

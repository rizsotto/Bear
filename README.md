Build EAR
=========

Bear is a tool to generate compilation database for clang tooling.

The [JSON compilation database][1] is used in clang project to provide
information how a single compilation unit was processed. When that
is available then it is easy to re-run the compilation with different
compiler. Or even more, it can re-run multiple of these compilation in
one executable. (Look for clang tooling capabilities.)

One format of these compilation database, comming from `cmake`. Passing
`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON` to cmake generates `compile_commands.json`
file into the current directory.

When the project compiles with no cmake, but another build system, there is
no free json file. Bear is a tool to generate such file during the build
process.

The concept behind Bear is to exec the original build command and
intercept the `exec` calls. To achive that Bear uses `LD_PRELOAD` mechanism
provided by GNU C library. So it has two components: one is the library which
defines the `exec` methods and used in every child processes, second is the
executable which set the environment up to child processes.


How to build
------------

* It is better to build it in a separate build directory.
`mkdir build && cd build`
* The configure step made by cmake: `cmake ..`
You can pass `-DCMAKE_INSTALL_PREFIX=<path>` to override the default
`/usr/local`. For more cmake control, read about the [related variables][2].
* To compile and run test suite: `make check`
* To install: `make install` You can specify `DESTDIR` environment to prefix
the `CMAKE_INSTALL_PREFIX`.


How to use
----------

The usage is like this

```shell
$ bear -o commands.json -- make
```

The `-o` option specify the output file, while the `--` separate the parameters
from the build command.

Known issues
------------

Compiler wrappers like [ccache][3] and [distcc][4] could cause duplicates
or missing items in the compilation database. Make sure you have been disabled
before you run Bear.


[1]: http://clang.llvm.org/docs/JSONCompilationDatabase.html
[2]: http://www.cmake.org/Wiki/CMake_Useful_Variables
[3]: http://ccache.samba.org/
[4]: http://code.google.com/p/distcc/

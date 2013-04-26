Build EAR
=========

Bear is a tool to generate compilation database for clang tooling.

The [JSON compilation database][1] is used in clang project to provide
information how a single compilation unit was processed. When that
is available then it is easy to re-run the compilation with different
programs.

One way to get compilation database is to use `cmake` as build tool. Passing
`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON` to cmake generates `compile_commands.json`
file into the current directory.

When the project compiles with no cmake, but another build system, there is
no free json file. Bear is a tool to generate such file during the build
process.

The concept behind Bear is to exec the original build command and
intercept the `exec` calls of the build tool. To achive that Bear uses the
`LD_PRELOAD` or `DYLD_INSERT_LIBRARIES` mechanisms provided by the dynamic
linker. So it has two components: the library and the binary. The library
defines the `exec` methods and used in every child processes. The executable
sets the environment up to child processes and writes the output file.


How to build
------------

You need a C compiler and cmake installed. To create man page, you need
*pandoc* installed on your system. To create packages there are targets
in the cmake file.

* It is better to build it in a separate build directory.
`mkdir build && cd build`
* The configure step made by cmake: `cmake ..`
You can pass `-DCMAKE_INSTALL_PREFIX=<path>` to override the default
`/usr/local`. For more cmake control, read about the [related variables][2].
* To install: `make install` You can specify `DESTDIR` environment to prefix
the `CMAKE_INSTALL_PREFIX`.


How to use
----------

The usage is like this

```shell
$ bear -- make
```

The `--` separate the parameters from the build command. The output file
called `compile_commands.json` found  in current directory.


Known issues
------------

Compiler wrappers like [ccache][3] and [distcc][4] could cause duplicates
or missing items in the compilation database. Make sure you have been disabled
before you run Bear.


[1]: http://clang.llvm.org/docs/JSONCompilationDatabase.html
[2]: http://www.cmake.org/Wiki/CMake_Useful_Variables
[3]: http://ccache.samba.org/
[4]: http://code.google.com/p/distcc/

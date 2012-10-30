Build EAR
=========

Bear is a tool to generate compilation database for clang tooling.

The [JSON compilation database][1] is used in clang project to provide
information how a single compilation unit was compiled. If that information
is available then it is easy to re-run the compilation with different
compiler. Or even more, it can re-run multiple of these compilation in
one executable. (Look for clang tooling capabilities.) But to generate such
compilation database is not easy, if the project is not using `cmake`,
which generates this kind of file.

The concept behind `bear` is to exec the original build command and
intercept the `exec` calls. To achive that `bear` uses `LD_PRELOAD` mechanism
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
$ bear -o output.cmake -- make
```

The `-o` option specify the output file, while the `--` separate the parameters
from the build command.

[1]: http://clang.llvm.org/docs/JSONCompilationDatabase.html
[2]: http://www.cmake.org/Wiki/CMake_Useful_Variables

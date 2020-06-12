[![Packaging status](https://repology.org/badge/tiny-repos/bear.svg)](https://repology.org/project/bear/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/continuous%20integration/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)
[![Gitter](https://img.shields.io/gitter/room/rizsotto/Bear)](https://gitter.im/rizsotto/Bear)

Build EAR (BEAR)
================

Bear is a tool that generates a compilation database for clang tooling.

The [JSON compilation database][JSONCDB] is used in the clang project
to provide information on how a single compilation unit is processed.
With this, it is easy to re-run the compilation with alternate
programs.

One way to get a compilation database is to use `cmake` as the build
tool. Passing `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON` to cmake generates
the `compile_commands.json` file into the current directory.

For non-cmake projects, Bear generates the JSON file during the build process.

The concept behind Bear is: to execute the original build command and
intercept the `exec` calls issued by the build tool. To achieve that,
Bear uses the `LD_PRELOAD` or `DYLD_INSERT_LIBRARIES` mechanisms provided
by the dynamic linker.

Bear has two components: the library and the binary. The library
redefines the `exec` methods to be used by all child processes. The
executable enables the use of the library for child processes and
writes the output file.

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html


How to install
--------------

Bear is packaged for many distributions. Check out your package manager.
Or build it from source.


How to build
------------

Bear should be quite portable on UNIX operating systems. It has been
tested on FreeBSD, GNU/Linux and OS X.

### Prerequisites

1. a **C++ compiler**, to compile the sources. (Shall support C++17 dialect.)
2. **CMake**, to configure the build. (Minimum version is 3.2) And a
   build tool [supported](https://cmake.org/cmake/help/v3.5/manual/cmake-generators.7.html)
   by CMake.
3. **protoc** and **grpc_cpp_plugin** commands. (See gRPC dependencies.)

### Dependencies

The dependencies can come from OS packages or the build will fetch the sources
and build locally.

- [python](https://www.python.org/) >= 3.5
- [gRPC](https://github.com/grpc/grpc) >= 1.26
- [fmt](https://github.com/fmtlib/fmt) >= 6.2
- [spdlog](https://github.com/gabime/spdlog) >= 1.5
- [json](https://github.com/nlohmann/json) >= 3.7

Developer dependencies:

- [googletest](https://github.com/google/googletest) >= 1.10
- [lit](https://pypi.org/project/lit/0.7.1/) >= 0.7

Install dependencies from packages on Fedora 32

    dnf install json-devel spdlog-devel fmt-devel grpc-devel grpc-plugins
    dnf install gtest-devel gmock-devel # optional for running the tests
    
Install dependencies from packages on Arch

    pacman -S grpc spdlog fmt nlohmann-json
    pacman -S gtest gmock # optional for running the tests

### Build commands

Ideally, you should build Bear in a separate build directory.

    cmake -DENABLE_UNIT_TESTS=OFF -DENABLE_FUNC_TESTS=OFF $BEAR_SOURCE_DIR
    make all
    make install

You can configure the build process with passing arguments to cmake.

To run test during the build process, you will need to install the
test frameworks and re-configure the build. For unit testing Bear
uses googletest, which will be built from source if you not install
it before.

    # install `lit` the functional test framework into a python virtualenv
    mkvirtualenv bear
    pip install lit
    # it's important to re-run the configure step again
    cmake $BEAR_SOURCE_DIR
    make all
    make check

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
need to run `make clean` before you run `bear make`.)

Known issues
------------

### Environment overriding caused problems

Because Bear uses `LD_PRELOAD` or `DYLD_INSERT_LIBRARIES` environment variables,
it does not append to it, but overrides it. So builds which are using these
variables might not work. (I don't know any build tool which does that, but
please let me know if you do.)

### Build with multiple architecture support

Multilib is one of the solutions allowing users to run applications built
for various application binary interfaces (ABIs) of the same architecture.
The most common use of multilib is to run 32-bit applications on 64-bit
kernel.

For OSX this is not an issue. The build commands from previous section will
work, Bear will intercept compiler calls for 32-bit and 64-bit applications.

For Linux, a small tune is needed at build time. Need to compile `libear.so`
library for 32-bit and for 64-bit too. Then install these libraries to the OS
preferred multilib directories. And replace the `libear.so` path default
value with a single path, which matches both. (The match can be achieved by
the `$LIB` token expansion from the dynamic loader. See `man ld.so` for more.)

Debian derivatives are using `lib/i386-linux-gnu` and `lib/x86_64-linux-gnu`,
while many other distributions are simple `lib` and `lib64`. Here comes an
example build script to install a multilib capable Bear. It will install Bear
under `/opt/bear` on a non Debian system.

    (cd ~/build32; cmake "$BEAR_SOURCE_DIR" -DCMAKE_C_COMPILER_ARG1="-m32"; VERBOSE=1 make all;)
    (cd ~/build64; cmake "$BEAR_SOURCE_DIR" -DCMAKE_C_COMPILER_ARG1="-m64" -DDEFAULT_PRELOAD_FILE='/opt/bear/$LIB/libear.so'; VERBOSE=1 make all;)
    sudo install -m 0644 ~/build32/libear/libear.so /opt/bear/lib/libear.so
    sudo install -m 0644 ~/build64/libear/libear.so /opt/bear/lib64/libear.so
    sudo install -m 0555 ~/build64/bear/bear" /opt/bear/bin/bear

To check you installation, install `lit` and run the test suite.

    PATH=/opt/bear/bin:$PATH lit -v test

### Empty compilation database on OS X / macOS or Fedora

Security extension/modes on different operating systems might disable library
preloads. In this case Bear behaves normally, but the result compilation database
will be empty. (Please make sure it's not the case when reporting bugs.)
Notable examples for enabled security modes are: OS X 10.11 (check with
`csrutil status | grep 'System Integrity Protection'`), and Fedora, CentOS, RHEL
(check with `sestatus | grep 'SELinux status'`).

Workaround could be to disable the security feature while running Bear. (This
might involve reboot of your computer, so might be heavy workaround.) Another
option if the build tool is not installed under [certain][osx_sip] directories.
Or use tools which are using compiler wrappers. (It injects a fake compiler
which does record the compiler invocation and calls the real compiler too.)
An example for such tool might be [scan-build][scanbuild]. The build system
shall respect `CC` and `CXX` environment variables.

  [osx_sip]: https://support.apple.com/en-us/HT204899
  [scanbuild]: https://github.com/rizsotto/scan-build

### Bazel builds produce empty outputs

The two main constraints to intercept compiler execution from bazel builds are:
bazel runs a daemon which runs the compilations, and it creates an isolated
environment to run the compiler. These problems are not just hard to circumvent,
but the workaround would not be stable to support it by this tool.

The good news is: there are extensions for bazel to generate the compilation
database.

### Static build tool produce empty output

Currently Bear based on dynamic linker load mechanism, executions made by
statically linked binaries are not captured. It means, if the build tool is
statically linked binary, compiler calls won't be recorded by Bear.

Problem reports
---------------

If you find a bug in this documentation or elsewhere in the program or would
like to propose an improvement, please use the project's [github issue
tracker][ISSUES]. Please describing the bug and where you found it. If you
have a suggestion how to fix it, include that as well. Patches are also
welcome.

Please follow the [contribution guide][GUIDE] when you do this.

  [ISSUES]: https://github.com/rizsotto/Bear/issues
  [GUIDE]: https://github.com/rizsotto/Bear/blob/master/.github/CONTRIBUTING.md

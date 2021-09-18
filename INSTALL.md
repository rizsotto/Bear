How to build
============

Bear should be quite portable on UNIX operating systems. It has been
tested on FreeBSD, GNU/Linux and OS X.

## Build dependencies

1. **C++ compiler**, to compile the sources. (Should support
   [C++17 dialect](https://en.cppreference.com/w/cpp/compiler_support#cpp17).)
2. **CMake**, to configure the build. (Minimum version is 3.12) And a
   build tool [supported](https://cmake.org/cmake/help/v3.5/manual/cmake-generators.7.html)
   by CMake.
3. **pkg-config** to look up dependencies' compiler flags.
4. **protoc** and **grpc_cpp_plugin** commands. (See gRPC dependencies.)

## Dependencies

The dependencies can come from OS packages or the build will fetch the sources
and build locally.

- [gRPC](https://github.com/grpc/grpc) >= 1.26
- [fmt](https://github.com/fmtlib/fmt) >= 6.2
- [spdlog](https://github.com/gabime/spdlog) >= 1.5
- [json](https://github.com/nlohmann/json) >= 3.7

Developer dependencies:

- [python](https://www.python.org/) >= 3.5
- [googletest](https://github.com/google/googletest) >= 1.10
- [lit](https://pypi.org/project/lit/0.7.1/) >= 0.7

## Build commands

Ideally, you should build Bear in a separate build directory.

    cmake -DENABLE_UNIT_TESTS=OFF -DENABLE_FUNC_TESTS=OFF $BEAR_SOURCE_DIR
    make all
    make install

You can configure the build process with passing arguments to cmake.
One of the flags you might want to pay attention is the `CMAKE_INSTALL_LIBDIR`
flag, which has to be the directory name for libraries. (The value of this
varies for different distribution: debian derivatives are using
`lib/i386-linux-gnu` and `lib/x86_64-linux-gnu`, while many other distributions
are simple `lib` and `lib64` directories.) Passing the flag looks like this:

    cmake -DCMAKE_INSTALL_LIBDIR=lib/x86_64-linux-gnu ... $BEAR_SOURCE_DIR

To run test during the build process, you will need to install the
test frameworks and re-configure the build. For unit testing Bear
uses googletest, which will be built from source if not already installed.

    # install `lit` the functional test framework into a python virtualenv
    mkvirtualenv bear
    pip install lit
    # it's important to re-run the configure step again
    cmake $BEAR_SOURCE_DIR
    cmake --build $build_dir --parallel 4

## OS specific notes

Install dependencies from packages on Fedora 32/33

    dnf install python cmake pkg-config
    dnf install json-devel spdlog-devel fmt-devel grpc-devel grpc-plugins
    dnf install gtest-devel gmock-devel # optional for running the tests
    
Install dependencies from packages on Arch

    pacman -S python cmake pkg-config
    pacman -S grpc spdlog fmt nlohmann-json
    pacman -S gtest gmock # optional for running the tests

Install dependencies from packages on Ubuntu 20.04

    apt-get install python cmake pkg-config
    apt-get install libfmt-dev libspdlog-dev nlohmann-json3-dev \
                    libgrpc++-dev protobuf-compiler-grpc libssl-dev

Install dependencies from packages from Brew

    brew install fmt spdlog nlohmann-json grpc pkg-config

Install dependencies from packages on Alpine edge

    apk add git cmake pkgconf make g++
    apk add fmt-dev spdlog-dev nlohmann-json protobuf-dev grpc-dev c-ares-dev

### Platform: macOS

Xcode < 11 or macOS < 10.15 users should get [LLVM Clang](https://releases.llvm.org)
binaries and headers. Make sure that `clang++ -v` returns the correct `InstalledDir`.
This is because `std::filesystem` is not available on Clang supplied with Xcode < 11,
and `std::filesystem::path` is not available in system C++ dylib for macOS < 10.15.

If OpenSSL is installed via Brew, and it's keg-only, run the following (before the
build) for pkg-config to find it as grpc's dependency:
    
    export PKG_CONFIG_PATH=$(brew --prefix)/opt/openssl@1.1/lib/pkgconfig


# Contains changes needed to make Bear usable on macOS versions < 10.15.

include(CheckIncludeFileCXX)
check_include_file_cxx(filesystem HAVE_FILESYSTEM)

if(NOT HAVE_FILESYSTEM)
  message(FATAL_ERROR "filesystem header NOT found. Please use LLVM Clang or upgrade to Xcode 11.")
endif()

# This needs some explaining.
# Clang shipped with Xcode < 11 does not have filesystem header. But developers
# can download LLVM toolchain which has the header, without upgrading anything.
# But the filesystem header in LLVM toolchain gives compile error if you use
# `std::filesystem::path` because the symbol is not present in macOS < 10.15's
# system C++ dynamic libraries. So:
# - Disable the check for the availability of a symbol because LLVM
#   toolchain's dylib definitely has it. The check is there only for vendors
#   of their version of dylibs.
# - Ship a dylib with Bear which contains the symbol so that users who have not
#   downloaded the LLVM toolchain can also launch the app.
#
# Read more about it in:
# https://github.com/llvm/llvm-project/blob/2eadbc86142bab5b46dfeb55d8bd6724234278bb/libcxx/include/__availability#L19

# Darwin 19 (macOS 10.15) adds the std::filesystem::path.
if(CMAKE_SYSTEM_VERSION VERSION_LESS 19)
    # This fixes the "path explicitly marked unavailable here" error.
    add_definitions("-D_LIBCPP_DISABLE_AVAILABILITY")
    # Final location for clang dylibs and aliases.
    set(INSTALL_PATH_LIBCPPDYLIB "${ROOT_INSTALL_PREFIX}/libexec")
    get_filename_component(CLANG_INSTALL_PREFIX ${CMAKE_CXX_COMPILER}/../../ ABSOLUTE)
    # _Users_ (not developers) may not have the LLVM dylib on their system. So find the
    # symbol in the dylibs we ship relative to the binary.
    execute_process(
        COMMAND ${CMAKE_COMMAND} -E copy
            ${CLANG_INSTALL_PREFIX}/lib/libc++.1.0.dylib
            ${CLANG_INSTALL_PREFIX}/lib/libc++.1.dylib
            ${CLANG_INSTALL_PREFIX}/lib/libc++.dylib

            "${INSTALL_PATH_LIBCPPDYLIB}"
        ERROR_VARIABLE ERROR_COPY_LIB
    )
    if(ERROR_COPY_LIB)
        message(WARNING "${ERROR_COPY_LIB}")
    endif()
    # https://libcxx.llvm.org/docs/UsingLibcxx.html#alternate-libcxx
    # Don't use system C++ dylib and headers, but the one LLVM supplies.
    add_compile_options("-stdlib=libc++;-nostdinc++;-I${CLANG_INSTALL_PREFIX}/include/c++/v1")
    # For developers, executables may be deep inside some folder created by CMake
    # while building. So use a constant location for finding dylibs.
    add_link_options("-L${CLANG_INSTALL_PREFIX}/lib;-Wl,-rpath,${INSTALL_PATH_LIBCPPDYLIB}")
    # For users, find the dylibs where they were copied earlier.
    add_link_options("-Wl,-rpath,@executable_path/../libexec")
endif()

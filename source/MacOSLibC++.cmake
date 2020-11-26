if( NOT APPLE)
    message(FATAL_ERROR "Don't include this on non-macOS platforms.")
endif()

include(CheckIncludeFileCXX)
check_include_file_cxx(filesystem HAVE_FILESYSTEM)

if(NOT HAVE_FILESYSTEM)
  message(FATAL_ERROR "filesystem header NOT found. Please use LLVM Clang or upgrade to Xcode 11.")
endif()

# Darwin 19 (macOS 10.15) adds the std::filesystem::path.
if(CMAKE_SYSTEM_VERSION VERSION_LESS 19)
    # This fixes the "path explicitly marked unavailable here" error.
    add_definitions("-D_LIBCPP_DISABLE_AVAILABILITY")
    # Final location for clang dylibs and aliases.
    set(INSTALL_PATH_LIBCPPDYLIB "${ROOT_INSTALL_PREFIX}/libexec")
    get_filename_component(CLANG_INSTALL_PREFIX ${CMAKE_CXX_COMPILER}/../../ ABSOLUTE)
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
    # Don't use system C++ dylib, but the one LLVM supplies.
    add_compile_options("-stdlib=libc++;-nostdinc++;-I${CLANG_INSTALL_PREFIX}/include/c++/v1")
    # For linking and testing, executables should find the dylibs here.
    add_link_options("-L${CLANG_INSTALL_PREFIX}/lib;-Wl,-rpath,${INSTALL_PATH_LIBCPPDYLIB}")
    # For distribution and running from installed directories, executables should find the dylibs here.
    add_link_options("-Wl,-rpath,@executable_path/../libexec")
endif()

macro(remove_temporary_rpath
    target_name
    )

    add_custom_command(TARGET ${target_name}
        COMMAND xcrun install_name_tool -delete_rpath ${INSTALL_PATH_LIBCPPDYLIB} $<TARGET_FILE:${target_name}>
        POST_BUILD
    )
endmacro()

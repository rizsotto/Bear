# Create a header file with the relevant configuration values.
include(CheckIncludeFile)
check_include_file(spawn.h HAVE_SPAWN_H)
check_include_file(dlfcn.h HAVE_DLFCN_H)
check_include_file(errno.h HAVE_ERRNO_H)
check_include_file(unistd.h HAVE_UNISTD_H)
include(CheckSymbolExists)
set(CMAKE_REQUIRED_LIBRARIES ${CMAKE_DL_LIBS})
set(CMAKE_REQUIRED_FLAGS -D_GNU_SOURCE)
check_symbol_exists(_NSGetEnviron "crt_externs.h" HAVE_NSGETENVIRON)
check_symbol_exists(dlsym "dlfcn.h" HAVE_DLSYM)
check_symbol_exists(RTLD_NEXT "dlfcn.h" HAVE_RTLD_NEXT)
check_symbol_exists(EACCES "errno.h" HAVE_EACCES)
check_symbol_exists(ENOENT "errno.h" HAVE_ENOENT)

configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)

# Create a static library, which is used for unit tests and the final shared library.
add_library(exec_a STATIC
        config.h
        ../include/libexec.h
        Array.h
        Buffer.h
        Buffer.cc
        Environment.h
        Environment.cc
        Executor.h
        Executor.cc
        Logger.h
        Logger.cc
        Resolver.h
        Resolver.cc
        Session.h
        Session.cc
        )

target_include_directories(exec_a PRIVATE ${LIB_CFG_DIR})
target_include_directories(exec_a PRIVATE ${LIB_API_DIR})
target_include_directories(exec_a PRIVATE ${EXE_API_DIR})
target_link_libraries(exec_a ${CMAKE_DL_LIBS})
target_compile_features(exec_a PUBLIC cxx_std_14)
target_compile_options(exec_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(exec_a PROPERTIES
        LINKER_LANGUAGE "C"
        POSITION_INDEPENDENT_CODE 1)

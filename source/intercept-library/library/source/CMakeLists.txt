# Create a header file with the relevant configuration values.
include(CheckSymbolExists)
include(CheckIncludeFile)
check_symbol_exists(_NSGetEnviron crt_externs.h HAVE_NSGETENVIRON)
check_include_file(spawn.h HAVE_SPAWN_HEADER)

configure_file(config.h.in ${CMAKE_CURRENT_BINARY_DIR}/config.h)

# Create a static library, which is used for unit tests and the final shared library.
add_library(exec_a STATIC
        config.h
        ../api/libexec.h
        Array.h
        Environment.h
        Environment.cc
        Executor.h
        Executor.cc
        Resolver.h
        Session.h
        Session.cc
        Storage.h
        Storage.cc
)

target_include_directories(exec_a PRIVATE ${LIB_CFG_DIR})
target_include_directories(exec_a PRIVATE ${LIB_API_DIR})
target_include_directories(exec_a PRIVATE ${EXE_API_DIR})
target_compile_features(exec_a PUBLIC cxx_std_14)
target_compile_options(exec_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(exec_a PROPERTIES
        LINKER_LANGUAGE "C"
        POSITION_INDEPENDENT_CODE 1)

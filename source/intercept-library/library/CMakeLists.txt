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

# Create library for the interface only
add_library(exec_api INTERFACE
        )

target_include_directories(exec_api INTERFACE include/)

# Create library for test and te final target
add_library(exec_a OBJECT
        source/Buffer.cc
        source/Environment.cc
        source/Executor.cc
        source/Logger.cc
        source/Resolver.cc
        source/Session.cc
        )

target_include_directories(exec_a PUBLIC ${CMAKE_CURRENT_BINARY_DIR})
target_include_directories(exec_a PUBLIC source)
target_link_libraries(exec_a PUBLIC exec_api)
target_link_libraries(exec_a PUBLIC er_api)
target_link_libraries(exec_a PUBLIC ${CMAKE_DL_LIBS})
target_compile_features(exec_a PUBLIC cxx_std_14)
target_compile_options(exec_a PUBLIC -fno-exceptions -fno-rtti)
set_target_properties(exec_a PROPERTIES
        LINKER_LANGUAGE "C"
        POSITION_INDEPENDENT_CODE 1)

# Create a shared library from the sub projects.
add_library(exec SHARED
        lib.cc
        )

target_link_libraries(exec exec_a)
set_target_properties(exec PROPERTIES
        LINKER_LANGUAGE "C"
        OSX_ARCHITECTURES:STRING "i386;x86_64"
        MACOSX_RPATH:BOOL ON
        )

if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(CMAKE_CXX_IMPLICIT_LINK_LIBRARIES "")
    set_target_properties(exec PROPERTIES LINK_FLAGS "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/libexec.version")
endif ()

# Create unit test.
add_executable(exec_unit_test
        test/ArrayTest.cc
        test/BufferTest.cc
        test/ExecutorTest.cc
        test/SessionTest.cc
        )

target_include_directories(exec_unit_test PRIVATE test)
target_link_libraries(exec_unit_test exec_api)
target_link_libraries(exec_unit_test exec_a)
target_link_libraries(exec_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(exec_unit_test GTest::gmock)

add_test(NAME bear::exec_unit_test COMMAND exec_unit_test)

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

include(GNUInstallDirs)
install(TARGETS exec
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})

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

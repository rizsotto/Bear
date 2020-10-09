# Create library for the interface only
add_library(exec_api INTERFACE
        )

target_include_directories(exec_api INTERFACE include/)

# Create library for test and te final target
add_library(exec_a OBJECT
        source/Buffer.cc
        source/Environment.cc
        source/Executor.cc
        source/Linker.cc
        source/Logger.cc
        source/Paths.cc
        source/Resolver.cc
        source/Session.cc
        )

target_include_directories(exec_a PUBLIC source)
target_link_libraries(exec_a PUBLIC exec_api)
target_link_libraries(exec_a PUBLIC er_api)
target_link_libraries(exec_a PUBLIC ${CMAKE_DL_LIBS})
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
if (ENABLE_UNIT_TESTS)
    add_executable(exec_unit_test
            test/ArrayTest.cc
            test/BufferTest.cc
            test/PathsTest.cc
            test/SessionTest.cc
            )

    target_include_directories(exec_unit_test PRIVATE test)
    target_link_libraries(exec_unit_test exec_api)
    target_link_libraries(exec_unit_test exec_a)
    target_link_libraries(exec_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::exec_unit_test COMMAND $<TARGET_FILE:exec_unit_test>)
endif ()
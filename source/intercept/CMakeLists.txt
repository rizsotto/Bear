add_subdirectory(proto)

include(GNUInstallDirs)

add_library(report_a OBJECT
        source/collect/EventsDatabase.cc
        )

target_include_directories(report_a PUBLIC include)
target_link_libraries(report_a PUBLIC result_a)
target_link_libraries(report_a PUBLIC fmt::fmt)
target_link_libraries(report_a PUBLIC spdlog::spdlog)
target_link_libraries(report_a PUBLIC nlohmann_json::nlohmann_json)
target_link_libraries(report_a PUBLIC rpc_a)
target_link_libraries(report_a PUBLIC PkgConfig::sqlite)

# Create a static library, which is used for unit tests and the final shared library.
add_library(intercept_a OBJECT
        source/collect/Application.cc
        source/collect/Reporter.cc
        source/collect/RpcServices.cc
        source/collect/Session.cc
        source/collect/SessionWrapper.cc
        )

if (SUPPORT_PRELOAD)
    target_sources(intercept_a PRIVATE source/collect/SessionLibrary.cc)
endif()

target_include_directories(intercept_a PUBLIC include/)
target_include_directories(intercept_a PUBLIC source/)
target_link_libraries(intercept_a PUBLIC main_a)
target_link_libraries(intercept_a PUBLIC exec_a)
target_link_libraries(intercept_a PUBLIC flags_a)
target_link_libraries(intercept_a PUBLIC rpc_a)
target_link_libraries(intercept_a PUBLIC sys_a)
target_link_libraries(intercept_a PUBLIC report_a)
target_link_libraries(intercept_a PUBLIC result_a)
target_link_libraries(intercept_a PUBLIC spdlog::spdlog)
target_link_libraries(intercept_a PUBLIC stdc++fs)

# Create an executable from the sub projects.
add_executable(intercept
        source/collect/main.cc
        )

target_link_libraries(intercept main_a)
target_link_libraries(intercept intercept_a)
target_link_libraries(intercept report_a)
target_link_libraries(intercept rpc_a)
target_link_libraries(intercept sys_a)
target_link_libraries(intercept flags_a)
target_link_libraries(intercept exec_a)

install(TARGETS intercept
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

# Markdown file is the source to the man file. Please modify that and generate
# the man file from it with pandoc.
#
#   $ pandoc -s -t man intercept.1.md -o intercept.1
#
# This is not automated, because pandoc has big dependencies on different OS
# distributions and packaging would require to install those. Which might be
# too much effort to generate a single text file.

install(FILES man/intercept.1
        DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)

add_executable(wrapper
        source/report/wrapper/EventFactory.cc
        source/report/wrapper/RpcClients.cc
        source/report/wrapper/Application.cc
        source/report/wrapper/main.cc
        )

target_include_directories(wrapper PUBLIC source/)
target_link_libraries(wrapper main_a)
target_link_libraries(wrapper flags_a)
target_link_libraries(wrapper result_a)
target_link_libraries(wrapper sys_a)
target_link_libraries(wrapper rpc_a)
target_link_libraries(wrapper spdlog::spdlog)
target_link_libraries(wrapper stdc++fs)

install(TARGETS wrapper
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})
install(DIRECTORY
        DESTINATION ${CMAKE_INSTALL_LIBEXECDIR}/wrapper.d)
install(CODE "
    execute_process(
        COMMAND ln -sf ../wrapper ar
        COMMAND ln -sf ../wrapper as
        COMMAND ln -sf ../wrapper cc
        COMMAND ln -sf ../wrapper c++
        COMMAND ln -sf ../wrapper cpp
        COMMAND ln -sf ../wrapper gcc
        COMMAND ln -sf ../wrapper g++
        COMMAND ln -sf ../wrapper f77
        COMMAND ln -sf ../wrapper m2c
        COMMAND ln -sf ../wrapper pc
        COMMAND ln -sf ../wrapper lex
        COMMAND ln -sf ../wrapper flex
        COMMAND ln -sf ../wrapper yacc
        COMMAND ln -sf ../wrapper bison
        COMMAND ln -sf ../wrapper lint
        COMMAND ln -sf ../wrapper makeinfo
        COMMAND ln -sf ../wrapper tex
        COMMAND ln -sf ../wrapper tex2dvi
        COMMAND ln -sf ../wrapper weave
        COMMAND ln -sf ../wrapper cweave
        COMMAND ln -sf ../wrapper tangle
        COMMAND ln -sf ../wrapper ctangle
        COMMAND ln -sf ../wrapper nm
        COMMAND ln -sf ../wrapper ld
        COMMAND ln -sf ../wrapper strip
        COMMAND ln -sf ../wrapper objcopy
        COMMAND ln -sf ../wrapper objdump
        COMMAND ln -sf ../wrapper ranlib
        COMMAND ln -sf ../wrapper readelf
        WORKING_DIRECTORY ${DESTDIR}${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBEXECDIR}/wrapper.d
    )
")

add_library(exec_a OBJECT
        source/report/libexec/Buffer.cc
        source/report/libexec/Environment.cc
        source/report/libexec/Executor.cc
        source/report/libexec/Linker.cc
        source/report/libexec/Logger.cc
        source/report/libexec/Paths.cc
        source/report/libexec/Resolver.cc
        source/report/libexec/Session.cc
        )

target_include_directories(exec_a PUBLIC source)
target_link_libraries(exec_a PUBLIC result_a)
target_link_libraries(exec_a PUBLIC ${CMAKE_DL_LIBS})
set_target_properties(exec_a PROPERTIES
        LINKER_LANGUAGE "C"
        POSITION_INDEPENDENT_CODE 1)

# Create a shared library from the sub projects.
add_library(exec SHARED
        source/report/libexec/lib.cc
        source/report/libexec/std.cc
        )

target_link_libraries(exec exec_a)
if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set_target_properties(exec PROPERTIES
            LINKER_LANGUAGE "C"
            CMAKE_CXX_IMPLICIT_LINK_LIBRARIES ""
            LINK_FLAGS "-Wl,--version-script=${CMAKE_CURRENT_SOURCE_DIR}/libexec.version"
            )
elseif (CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    set_target_properties(exec PROPERTIES
            LINKER_LANGUAGE "C"
            OSX_ARCHITECTURES:STRING "i386;x86_64"
            MACOSX_RPATH:BOOL ON
            )
endif ()

install(TARGETS exec
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})

# Create unit test.
if (ENABLE_UNIT_TESTS)
    add_executable(exec_unit_test
            test/libexec/ArrayTest.cc
            test/libexec/BufferTest.cc
            test/libexec/ExecutorTest.cc
            test/libexec/PathsTest.cc
            test/libexec/SessionTest.cc
            )

    target_include_directories(exec_unit_test PRIVATE test)
    target_link_libraries(exec_unit_test exec_a)
    target_link_libraries(exec_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::exec_unit_test COMMAND $<TARGET_FILE:exec_unit_test>)

    add_executable(intercept_unit_test
            test/SessionTest.cc
            )

    target_link_libraries(intercept_unit_test main_a)
    target_link_libraries(intercept_unit_test exec_a)
    target_link_libraries(intercept_unit_test intercept_a)
    target_link_libraries(intercept_unit_test report_a)
    target_link_libraries(intercept_unit_test rpc_a)
    target_link_libraries(intercept_unit_test sys_a)
    target_link_libraries(intercept_unit_test flags_a)
    target_link_libraries(intercept_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::intercept_unit_test COMMAND $<TARGET_FILE:intercept_unit_test>)
endif ()

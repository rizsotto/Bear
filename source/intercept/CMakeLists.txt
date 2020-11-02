# Create a static library, which is used for unit tests and the final shared library.
add_library(intercept_a OBJECT
        source/collect/Application.cc
        source/collect/Reporter.cc
        source/collect/Services.cc
        source/collect/Session.cc
        source/collect/SessionWrapper.cc
        )

if (SUPPORT_PRELOAD)
    target_sources(intercept_a PRIVATE source/collect/SessionLibrary.cc)
endif()

target_include_directories(intercept_a PUBLIC source/)
target_link_libraries(intercept_a PUBLIC result_a)
target_link_libraries(intercept_a PUBLIC flags_a)
target_link_libraries(intercept_a PUBLIC rpc_a)
target_link_libraries(intercept_a PUBLIC sys_a)
target_link_libraries(intercept_a PUBLIC report_a)
target_link_libraries(intercept_a PUBLIC spdlog::spdlog)
target_link_libraries(intercept_a PUBLIC wrapper_api)
if (SUPPORT_PRELOAD)
    target_link_libraries(intercept_a PUBLIC er_api)
    target_link_libraries(intercept_a PUBLIC exec_api)
endif()

# Create an executable from the sub projects.
add_executable(intercept
        source/collect/main.cc
        )

target_link_libraries(intercept intercept_a)
target_link_libraries(intercept report_a)
target_link_libraries(intercept rpc_a)
target_link_libraries(intercept sys_a)
target_link_libraries(intercept flags_a)

include(GNUInstallDirs)
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

# Create unit test
if (ENABLE_UNIT_TESTS)
    add_executable(intercept_unit_test
            test/ReporterTest.cc
            test/SessionTest.cc
            )

    target_link_libraries(intercept_unit_test intercept_a)
    target_link_libraries(intercept_unit_test report_a)
    target_link_libraries(intercept_unit_test rpc_a)
    target_link_libraries(intercept_unit_test sys_a)
    target_link_libraries(intercept_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::intercept_unit_test COMMAND $<TARGET_FILE:intercept_unit_test>)
endif ()

# Create library for the interface only
add_library(wrapper_api INTERFACE
        )

target_include_directories(wrapper_api INTERFACE include/)

# Create a static library, which is used for unit tests and the final shared library.
add_library(wrapper_a OBJECT
        source/report/wrapper/Application.cc
        )

target_include_directories(wrapper_a PUBLIC source/)
target_link_libraries(wrapper_a PUBLIC wrapper_api)
target_link_libraries(wrapper_a PUBLIC result_a)
target_link_libraries(wrapper_a PRIVATE sys_a)
target_link_libraries(wrapper_a PRIVATE rpc_a)
target_link_libraries(wrapper_a PUBLIC spdlog::spdlog)
set_target_properties(wrapper_a PROPERTIES
        LINKER_LANGUAGE "C"
        )

# Create an executable from the sub projects.
add_executable(wrapper
        source/report/wrapper/main.cc
        )

target_link_libraries(wrapper wrapper_a)
target_link_libraries(wrapper sys_a)
target_link_libraries(wrapper rpc_a)
target_link_libraries(wrapper spdlog::spdlog)
set_target_properties(wrapper PROPERTIES
        LINKER_LANGUAGE "C"
        )

include(GNUInstallDirs)
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

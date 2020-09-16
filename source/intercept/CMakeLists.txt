# Create a static library, which is used for unit tests and the final shared library.
add_library(intercept_a OBJECT
        source/Application.cc
        source/Reporter.cc
        source/Services.cc
        source/Session.cc
        source/SessionWrapper.cc
        )

if (SUPPORT_PRELOAD)
    target_sources(intercept_a PRIVATE source/SessionLibrary.cc)
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
        main.cc
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

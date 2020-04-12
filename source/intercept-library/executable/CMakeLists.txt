# Create library for the interface only
add_library(er_api INTERFACE
        )

target_include_directories(er_api INTERFACE include/)

# Create a static library, which is used for unit tests and the final shared library.
add_library(er_a OBJECT
        source/Application.cc
        source/Environment.cc
        source/Reporter.cc
        source/SystemCalls.cc
        )

target_include_directories(er_a PUBLIC source/)
target_link_libraries(er_a PUBLIC er_api)
target_link_libraries(er_a PUBLIC exec_api)
target_link_libraries(er_a PUBLIC result_a)
target_link_libraries(er_a PUBLIC flags_a)
target_link_libraries(er_a PRIVATE sys_a)
target_link_libraries(er_a PRIVATE supervise_a)
target_link_libraries(er_a PUBLIC spdlog::spdlog)
set_target_properties(er_a PROPERTIES
        LINKER_LANGUAGE "C"
        )

# Create a static executable from the sub projects.
add_executable(er
        main.cc
        )

#target_link_libraries(er -static)
target_link_libraries(er ${CMAKE_DL_LIBS}) # TODO: remove it when static linking is done
target_link_libraries(er er_a)
target_link_libraries(er flags_a)
target_link_libraries(er sys_a)
target_link_libraries(er spdlog::spdlog)
target_link_libraries(er supervise_a)
set_target_properties(er PROPERTIES
        LINKER_LANGUAGE "C"
        )

include(GNUInstallDirs)
install(TARGETS er
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})

# Create unit test
add_executable(er_unit_test
        test/EnvironmentTest.cc
        test/ApplicationTest.cc
        )

target_link_libraries(er_unit_test er_a)
target_link_libraries(er_unit_test result_a)
target_link_libraries(er_unit_test flags_a)
target_link_libraries(er_unit_test sys_a)
target_link_libraries(er_unit_test supervise_a)
target_link_libraries(er_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(er_unit_test GTest::gmock)

add_test(NAME bear::er_unit_test COMMAND er_unit_test)

# Markdown file is the source to the man file. Please modify that and generate
# the man file from it with pandoc.
#
#   $ pandoc -s -t man man/er.1.md -o man/er.1
#
# This is not automated, because pandoc has big dependencies on different OS
# distributions and packaging would require to install those. Which might be
# too much effort to generate a single text file.

include(GNUInstallDirs)
install(FILES man/er.1
        DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)

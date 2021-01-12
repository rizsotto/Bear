# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_json_a OBJECT
        source/Configuration.cc
        source/Output.cc
        source/semantic/Command.cc
        )

target_link_libraries(citnames_json_a PUBLIC result_a)
target_link_libraries(citnames_json_a PUBLIC report_a)
target_link_libraries(citnames_json_a PUBLIC shell_a)
target_link_libraries(citnames_json_a PUBLIC sys_a)
target_link_libraries(citnames_json_a PUBLIC fmt::fmt)
target_link_libraries(citnames_json_a PUBLIC spdlog::spdlog)
target_link_libraries(citnames_json_a PRIVATE nlohmann_json::nlohmann_json)
target_compile_options(citnames_json_a PRIVATE -fexceptions)

# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_a OBJECT
        source/Application.cc
        source/semantic/Parsers.cc
        source/semantic/Semantic.cc
        source/semantic/Tool.cc
        source/semantic/ToolClang.cc
        source/semantic/ToolCuda.cc
        source/semantic/ToolGcc.cc
        source/semantic/ToolWrapper.cc
        source/semantic/ToolExtendingWrapper.cc
        )

target_include_directories(citnames_a PUBLIC include/)
target_include_directories(citnames_a PUBLIC source/)
target_link_libraries(citnames_a PUBLIC main_a)
target_link_libraries(citnames_a PUBLIC result_a)
target_link_libraries(citnames_a PUBLIC flags_a)
target_link_libraries(citnames_a PUBLIC report_a)
target_link_libraries(citnames_a PUBLIC sys_a)
target_link_libraries(citnames_a PUBLIC fmt::fmt)
target_link_libraries(citnames_a PUBLIC spdlog::spdlog)

# Create an executable from the sub projects.
add_executable(citnames
        main.cc
        )

target_link_libraries(citnames main_a)
target_link_libraries(citnames citnames_a)
target_link_libraries(citnames citnames_json_a)
target_link_libraries(citnames result_a)
target_link_libraries(citnames report_a)
target_link_libraries(citnames flags_a)
target_link_libraries(citnames shell_a)
target_link_libraries(citnames sys_a)
target_link_libraries(citnames rpc_a)
target_link_libraries(citnames fmt::fmt)
target_link_libraries(citnames spdlog::spdlog)
target_link_libraries(citnames nlohmann_json::nlohmann_json)

include(GNUInstallDirs)
install(TARGETS citnames
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

# Markdown file is the source to the man file. Please modify that and generate
# the man file from it with pandoc.
#
#   $ pandoc -s -t man citnames.1.md -o citnames.1
#
# This is not automated, because pandoc has big dependencies on different OS
# distributions and packaging would require to install those. Which might be
# too much effort to generate a single text file.

install(FILES man/citnames.1
        DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)

if (ENABLE_UNIT_TESTS)
    add_executable(citnames_unit_test
            test/OutputTest.cc
            test/ToolTest.cc
            test/ToolGccTest.cc
            )

    target_link_libraries(citnames_unit_test main_a)
    target_link_libraries(citnames_unit_test citnames_a)
    target_link_libraries(citnames_unit_test citnames_json_a)
    target_link_libraries(citnames_unit_test result_a)
    target_link_libraries(citnames_unit_test report_a)
    target_link_libraries(citnames_unit_test flags_a)
    target_link_libraries(citnames_unit_test shell_a)
    target_link_libraries(citnames_unit_test sys_a)
    target_link_libraries(citnames_unit_test rpc_a)
    target_link_libraries(citnames_unit_test fmt::fmt)
    target_link_libraries(citnames_unit_test nlohmann_json::nlohmann_json)
    target_link_libraries(citnames_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::citnames_unit_test COMMAND $<TARGET_FILE:citnames_unit_test>)
endif ()

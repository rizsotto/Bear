# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_json_a OBJECT)
target_sources(citnames_json_a
        PRIVATE
            source/Configuration.cc
            source/Output.cc
        INTERFACE
            $<TARGET_OBJECTS:citnames_json_a>
        )
target_link_libraries(citnames_json_a PUBLIC
        result_a
        shell_a
        sys_a
        fmt::fmt
        spdlog::spdlog
        nlohmann_json::nlohmann_json)
target_compile_options(citnames_json_a PRIVATE -fexceptions)

# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_a OBJECT)
target_include_directories(citnames_a PUBLIC source/ include/)
target_sources(citnames_a
        PRIVATE
            source/Citnames.cc
            source/semantic/Build.cc
            source/semantic/Common.cc
            source/semantic/Parsers.cc
            source/semantic/Semantic.cc
            source/semantic/ToolAny.cc
            source/semantic/ToolClang.cc
            source/semantic/ToolCuda.cc
            source/semantic/ToolGcc.cc
            source/semantic/ToolIntelFortran.cc
            source/semantic/ToolWrapper.cc
            source/semantic/ToolExtendingWrapper.cc
            source/semantic/ToolLinker.cc
            source/semantic/ToolAr.cc
        INTERFACE
            $<TARGET_OBJECTS:citnames_a>
        )
target_link_libraries(citnames_a PUBLIC
        main_a
        citnames_json_a
        events_db_a
        domain_a
        result_a
        flags_a
        sys_a
        exec_a
        fmt::fmt
        spdlog::spdlog)

include(GNUInstallDirs)

# Markdown file is the source to the man file. Please modify that and generate
# the man file from it with pandoc.
#
#   $ pandoc -s -t man bear-citnames.1.md -o bear-citnames.1
#
# This is not automated, because pandoc has big dependencies on different OS
# distributions and packaging would require to install those. Which might be
# too much effort to generate a single text file.

install(FILES man/bear-citnames.1
        DESTINATION ${CMAKE_INSTALL_MANDIR}/man1)

if (ENABLE_UNIT_TESTS)
    add_executable(citnames_unit_test
            test/OutputTest.cc
            test/ParserTest.cc
            test/ToolClangTest.cc
            test/ToolGccTest.cc
            test/ToolIntelFortranTest.cc
            test/ToolWrapperTest.cc
            )

    target_link_libraries(citnames_unit_test citnames_a)
    target_link_libraries(citnames_unit_test citnames_json_a)
    target_link_libraries(citnames_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::citnames_unit_test COMMAND $<TARGET_FILE:citnames_unit_test>)
endif ()

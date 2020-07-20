# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_json_a OBJECT
        source/Config.cc
        source/CompilationDatabase.cc
        )

target_link_libraries(citnames_json_a PUBLIC result_a)
target_link_libraries(citnames_json_a PUBLIC shell_a)
target_link_libraries(citnames_json_a PUBLIC spdlog::spdlog)
target_link_libraries(citnames_json_a PRIVATE nlohmann_json::nlohmann_json)
target_compile_options(citnames_json_a PRIVATE -fexceptions)

# Create a static library, which is used for unit tests and the final shared library.
add_library(citnames_a OBJECT
        source/Application.cc
        )

target_include_directories(citnames_a PUBLIC source/)
target_link_libraries(citnames_a PUBLIC result_a)
target_link_libraries(citnames_a PUBLIC flags_a)
target_link_libraries(citnames_a PUBLIC sys_a)
target_link_libraries(citnames_a PUBLIC spdlog::spdlog)
set_target_properties(citnames_a PROPERTIES
        LINKER_LANGUAGE "C"
        )

# Create an executable from the sub projects.
add_executable(citnames
        main.cc
        )

target_link_libraries(citnames citnames_a)
target_link_libraries(citnames citnames_json_a)
target_link_libraries(citnames result_a)
target_link_libraries(citnames flags_a)
target_link_libraries(citnames shell_a)
target_link_libraries(citnames sys_a)
target_link_libraries(citnames spdlog::spdlog)
target_link_libraries(citnames nlohmann_json::nlohmann_json)
set_target_properties(citnames PROPERTIES
        LINKER_LANGUAGE "C"
        )

include(GNUInstallDirs)
install(TARGETS citnames
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

if (ENABLE_UNIT_TESTS)
    add_executable(citnames_unit_test
            test/CompilationDatabaseTest.cc
            )

    target_link_libraries(citnames_unit_test citnames_a)
    target_link_libraries(citnames_unit_test citnames_json_a)
    target_link_libraries(citnames_unit_test result_a)
    target_link_libraries(citnames_unit_test shell_a)
    target_link_libraries(citnames_unit_test nlohmann_json::nlohmann_json)
    target_link_libraries(citnames_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::citnames_unit_test COMMAND $<TARGET_FILE:citnames_unit_test>)
endif ()

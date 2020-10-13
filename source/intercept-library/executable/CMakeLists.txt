# Create library for the interface only
add_library(er_api INTERFACE
        )

target_include_directories(er_api INTERFACE include/)

# Create a static library, which is used for unit tests and the final shared library.
add_library(er_a OBJECT
        source/Application.cc
        )

target_include_directories(er_a PUBLIC source/)
target_link_libraries(er_a PUBLIC er_api)
target_link_libraries(er_a PUBLIC exec_api)
target_link_libraries(er_a PUBLIC result_a)
target_link_libraries(er_a PUBLIC flags_a)
target_link_libraries(er_a PRIVATE sys_a)
target_link_libraries(er_a PRIVATE rpc_a)
target_link_libraries(er_a PUBLIC spdlog::spdlog)
set_target_properties(er_a PROPERTIES
        LINKER_LANGUAGE "C"
        )

# Create an executable from the sub projects.
add_executable(er
        main.cc
        )

target_link_libraries(er er_a)
target_link_libraries(er flags_a)
target_link_libraries(er sys_a)
target_link_libraries(er rpc_a)
target_link_libraries(er spdlog::spdlog)
set_target_properties(er PROPERTIES
        LINKER_LANGUAGE "C"
        )

if (SUPPORT_PRELOAD)
    include(GNUInstallDirs)
    install(TARGETS er
            RUNTIME DESTINATION ${CMAKE_INSTALL_LIBEXECDIR})
endif()

# Create unit test
if (ENABLE_UNIT_TESTS)
    add_executable(er_unit_test
            test/ApplicationTest.cc
            )

    target_link_libraries(er_unit_test er_a)
    target_link_libraries(er_unit_test result_a)
    target_link_libraries(er_unit_test flags_a)
    target_link_libraries(er_unit_test sys_a)
    target_link_libraries(er_unit_test rpc_a)
    target_link_libraries(er_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::er_unit_test COMMAND $<TARGET_FILE:er_unit_test>)
endif ()

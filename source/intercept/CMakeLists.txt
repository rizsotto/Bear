add_subdirectory(proto)

# Create a static library, which is used for unit tests and the final shared library.
add_library(intercept_a OBJECT
        source/Application.cc
        source/Interceptor.cc
        source/Reporter.cc
        source/Session.cc
        )

target_include_directories(intercept_a PUBLIC source/)
target_link_libraries(intercept_a PUBLIC supervise_a)
target_link_libraries(intercept_a PUBLIC er_api)
target_link_libraries(intercept_a PUBLIC exec_api)
target_link_libraries(intercept_a PUBLIC result_a)
target_link_libraries(intercept_a PUBLIC flags_a)
target_link_libraries(intercept_a PUBLIC sys_a)
target_link_libraries(intercept_a PUBLIC nlohmann_json::nlohmann_json)
target_link_libraries(intercept_a PUBLIC spdlog::spdlog)

# Create an executable from the sub projects.
add_executable(intercept
        main.cc
        )

target_link_libraries(intercept intercept_a)
target_link_libraries(intercept supervise_a)
target_link_libraries(intercept sys_a)
target_link_libraries(intercept flags_a)

include(GNUInstallDirs)
install(TARGETS intercept
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})

# Create unit test
add_executable(intercept_unit_test
        test/ReporterTest.cc
        )

target_link_libraries(intercept_unit_test intercept_a)
target_link_libraries(intercept_unit_test supervise_a)
target_link_libraries(intercept_unit_test sys_a)
target_link_libraries(intercept_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(intercept_unit_test GTest::gmock)

add_test(NAME bear::intercept_unit_test COMMAND intercept_unit_test)

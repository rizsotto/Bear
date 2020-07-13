add_library(report_a OBJECT
        source/Report.cc
        )

target_include_directories(report_a PUBLIC include/)
target_link_libraries(report_a PUBLIC nlohmann_json::nlohmann_json)

#if (ENABLE_UNIT_TESTS)
#    add_executable(sys_unit_test
#            test/EnvironmentTest.cc
#            test/ErrorsTest.cc
#            test/PathTest.cc
#            )
#
#    target_include_directories(sys_unit_test PRIVATE source/)
#    target_link_libraries(sys_unit_test report_a)
#    target_link_libraries(sys_unit_test result_a)
#    target_link_libraries(sys_unit_test fmt::fmt)
#    target_link_libraries(sys_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})
#
#    add_test(NAME bear::sys_unit_test COMMAND $<TARGET_FILE:sys_unit_test>)
#endif ()

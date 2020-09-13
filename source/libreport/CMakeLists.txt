add_library(report_a OBJECT
        source/Report.cc
        )

target_include_directories(report_a PUBLIC include/)
target_link_libraries(report_a PUBLIC result_a)
target_link_libraries(report_a PRIVATE fmt::fmt)
target_link_libraries(report_a PRIVATE nlohmann_json::nlohmann_json)
target_compile_options(report_a PRIVATE -fexceptions)

if (ENABLE_UNIT_TESTS)
    add_executable(report_unit_test
            test/ReportTest.cc
            )

    target_link_libraries(report_unit_test report_a)
    target_link_libraries(report_unit_test result_a)
    target_link_libraries(report_unit_test fmt::fmt)
    target_link_libraries(report_unit_test nlohmann_json::nlohmann_json)
    target_link_libraries(report_unit_test PkgConfig::GTest ${CMAKE_THREAD_LIBS_INIT})

    add_test(NAME bear::report_unit_test COMMAND $<TARGET_FILE:report_unit_test>)
endif ()

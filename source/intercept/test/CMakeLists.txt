add_executable(intercept_unit_test
        ReporterTest.cc
        SystemCallsTest.cc
        )

target_link_libraries(intercept_unit_test intercept_a)
target_link_libraries(intercept_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(intercept_unit_test GTest::gmock)

add_test(NAME bear::intercept_unit_test COMMAND intercept_unit_test)

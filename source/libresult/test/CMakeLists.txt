add_executable(result_unit_test
        ResultTest.cc
        )

target_include_directories(result_unit_test PRIVATE GTest::gtest)
target_include_directories(result_unit_test PRIVATE GMock::gmock)
target_link_libraries(result_unit_test result_a)
target_link_libraries(result_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(result_unit_test GTest::gmock)

add_test(NAME bear::result_unit_test COMMAND result_unit_test)

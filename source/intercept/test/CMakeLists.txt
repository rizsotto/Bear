add_executable(intercept_unit_test
        SystemCallsTest.cc
        )

target_include_directories(intercept_unit_test PRIVATE ${EXE_CFG_DIR})
target_include_directories(intercept_unit_test PRIVATE ${EXE_SRC_DIR})
target_include_directories(intercept_unit_test PRIVATE GTest::gtest)
target_include_directories(intercept_unit_test PRIVATE GMock::gmock)
target_link_libraries(intercept_unit_test er_a)
target_link_libraries(intercept_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(intercept_unit_test GTest::gmock)

add_test(NAME bear::intercept_unit_test COMMAND intercept_unit_test)

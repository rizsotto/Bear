add_executable(intercept_unit_test
        SystemCallsTest.cc
        )

target_include_directories(intercept_unit_test PRIVATE ${EXE_CFG_DIR})
target_include_directories(intercept_unit_test PRIVATE ${EXE_SRC_DIR})
target_link_libraries(intercept_unit_test er_a)
target_link_libraries(intercept_unit_test gtest gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME LibInterceptUnitTests COMMAND intercept_unit_test)

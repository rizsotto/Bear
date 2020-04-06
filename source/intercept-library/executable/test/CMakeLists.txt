add_executable(er_unit_test
        EnvironmentTest.cc
        CommandTest.cc
        )

target_include_directories(er_unit_test PRIVATE ${EXE_CFG_DIR})
target_include_directories(er_unit_test PRIVATE ${EXE_SRC_DIR})
target_include_directories(er_unit_test PRIVATE ${EXE_API_DIR})
target_include_directories(er_unit_test PRIVATE GTest::gtest)
target_include_directories(er_unit_test PRIVATE GMock::gmock)
target_link_libraries(er_unit_test er_a)
target_link_libraries(er_unit_test result_a)
target_link_libraries(er_unit_test flags_a)
target_link_libraries(er_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(er_unit_test GTest::gmock)

add_test(NAME bear::er_unit_test COMMAND er_unit_test)

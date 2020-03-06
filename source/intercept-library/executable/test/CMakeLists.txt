add_executable(er_unit_test
        EnvironmentTest.cc
        ResultTest.cc
        SessionTest.cc
        )

target_include_directories(er_unit_test PRIVATE ${EXE_CFG_DIR})
target_include_directories(er_unit_test PRIVATE ${EXE_SRC_DIR})
target_include_directories(er_unit_test PRIVATE ${EXE_API_DIR})
target_link_libraries(er_unit_test er_a)
target_link_libraries(er_unit_test gtest gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME bear::er_unit_test COMMAND er_unit_test)

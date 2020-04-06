add_executable(exec_unit_test
        ResolverMock.h
        ArrayTest.cc
        BufferTest.cc
        ExecutorTest.cc
        SessionTest.cc
        )

target_include_directories(exec_unit_test PRIVATE ${LIB_CFG_DIR})
target_include_directories(exec_unit_test PRIVATE ${LIB_SRC_DIR})
target_include_directories(exec_unit_test PRIVATE ${EXE_API_DIR})
target_include_directories(exec_unit_test PRIVATE GTest::gtest)
target_include_directories(exec_unit_test PRIVATE GMock::gmock)
target_link_libraries(exec_unit_test exec_a)
target_link_libraries(exec_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(exec_unit_test GTest::gmock)

add_test(NAME bear::exec_unit_test COMMAND exec_unit_test)

add_executable(libexec_unit_test
        ResolverMock.h
        ArrayTest.cc
        BufferTest.cc
        ExecutorTest.cc
        SessionTest.cc
        )

target_include_directories(libexec_unit_test PRIVATE ${LIB_CFG_DIR})
target_include_directories(libexec_unit_test PRIVATE ${LIB_SRC_DIR})
target_include_directories(libexec_unit_test PRIVATE ${EXE_API_DIR})
target_include_directories(libexec_unit_test PRIVATE GTest::gtest)
target_include_directories(libexec_unit_test PRIVATE GMock::gmock)
target_link_libraries(libexec_unit_test exec_a)
target_link_libraries(libexec_unit_test GTest::gtest GTest::gtest_main ${CMAKE_THREAD_LIBS_INIT})
target_link_libraries(libexec_unit_test GTest::gmock)

add_test(NAME bear::libexec_unit_test COMMAND libexec_unit_test)

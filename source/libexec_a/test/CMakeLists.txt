add_executable(libexec_unit_test
        ArrayTest.cc
        ExecutorTest.cc
        SessionTest.cc
        StorageTest.cc
)

target_link_libraries(libexec_unit_test exec_a)
target_link_libraries(libexec_unit_test gtest gtest_main ${CMAKE_THREAD_LIBS_INIT})

add_test(NAME LibExecUnitTests COMMAND libexec_unit_test)

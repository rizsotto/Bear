add_executable(libexec_unit_test
        ArrayTest.cc
        SessionTest.cc
        ExecutorTest.cc
        StorageTest.cc
)

target_link_libraries(libexec_unit_test exec_a)
target_link_libraries(libexec_unit_test gtest gtest_main)

add_test(NAME LibExecUnitTests COMMAND libexec_unit_test)
